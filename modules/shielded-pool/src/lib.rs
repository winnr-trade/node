//! Shielded pool module for private balance management.
//!
//! This module enables users to deposit tokens into a shielded pool and withdraw
//! them privately using zero-knowledge proofs. Deposited tokens are held by the
//! pool and tracked via commitments in an incremental Merkle tree. Withdrawals
//! require proof of a valid commitment and consume a nullifier to prevent
//! double-spending.
//!
//! The primary use case for this module is to serve as the private collateral
//! management layer for the markets, allowing users to shield their
//! collateral and maintain privacy over their positions.

mod call;
mod error;
mod event;
mod genesis;
mod hash;
mod tree;
mod types;
pub mod verifier;

pub use call::CallMessage;
pub use error::ShieldedPoolError;
pub use event::{NoteKind, ShieldedPoolEvent};
pub use genesis::ShieldedPoolGenesisConfig;
pub use tree::{IncrementalMerkleTree, ZERO_LEAF};
pub use types::{ProofBytes, PublicInputs};

use crate::{call::MAX_MEMO_BYTES, error::IntoShieldedPoolError};
use sov_bank::{Amount, Bank, Coins, IntoPayable, Payable, TokenId};
use sov_chain_state::ChainState;
use sov_modules_api::{
    self, Context, EventEmitter, HexHash, Module, ModuleId, ModuleInfo, ModuleRestApi, SafeVec,
    Spec, StateMap, StateValue, TxState,
};

#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct ShieldedPoolModule<S: Spec> {
    #[id]
    pub id: ModuleId,

    /// Admin address with elevated permissions (e.g., managing supported tokens).
    #[state]
    pub admin: StateValue<S::Address>,

    /// Tracks whether an address has made its initial shielded deposit.
    /// Used by `ShieldFirst` to enforce one-time bootstrapping per address.
    #[state]
    pub has_shielded: StateMap<S::Address, bool>,

    /// Incremental Merkle tree storing deposit commitments.
    // TODO: IMT must be initialized during genesis (currently not set).
    #[state]
    pub tree: StateValue<IncrementalMerkleTree>,

    /// Set of spent nullifiers — prevents double-spending of commitments.
    #[state]
    pub nullifiers: StateMap<HexHash, bool>,

    /// Set of known commitments — prevents duplicate deposits.
    #[state]
    pub commitments: StateMap<HexHash, bool>,

    /// The single token accepted by this pool (same as the market collateral token).
    #[state]
    pub token_id: StateValue<TokenId>,

    #[module]
    pub bank: Bank<S>,

    #[module]
    pub chain_state: ChainState<S>,
}

impl<S: Spec> Module for ShieldedPoolModule<S> {
    type Spec = S;
    type Config = ShieldedPoolGenesisConfig<S>;
    type CallMessage = CallMessage;
    type Event = ShieldedPoolEvent;
    type Error = ShieldedPoolError;

    fn genesis(
        &mut self,
        header: &<<Self::Spec as Spec>::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl sov_modules_api::GenesisState<Self::Spec>,
    ) -> anyhow::Result<()> {
        self.init_module(header, config, state)
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        ctx: &Context<Self::Spec>,
        state: &mut impl TxState<Self::Spec>,
    ) -> Result<(), Self::Error> {
        match msg {
            CallMessage::CreateAccount {
                proof,
                root,
                amount,
                commitment,
                nullifier,
                memo,
            } => self.create_account(proof, root, amount, commitment, nullifier, memo, ctx, state),

            CallMessage::Deposit {
                proof,
                root,
                amount,
                commitment,
                nullifier,
                memo,
            } => self.deposit(proof, root, amount, commitment, nullifier, memo, ctx, state),

            CallMessage::Withdraw {
                proof,
                root,
                amount,
                commitment,
                nullifier,
                memo,
            } => self.withdraw(proof, root, amount, commitment, nullifier, memo, ctx, state),
        }
    }
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

impl<S: Spec> ShieldedPoolModule<S> {
    fn get_token_id(&self, state: &mut impl TxState<S>) -> Result<TokenId, ShieldedPoolError> {
        self.token_id
            .get(state)
            .into_shielded_pool_err()?
            .ok_or_else(|| ShieldedPoolError::Any(anyhow::anyhow!("token_id not initialized")))
    }

    fn create_account(
        &mut self,
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        if self
            .has_shielded
            .get(ctx.sender(), state)
            .into_shielded_pool_err()?
            .unwrap_or(false)
        {
            return Err(ShieldedPoolError::AlreadyShielded);
        }

        let token_id = self.get_token_id(state)?;
        if amount != Amount(0) {
            self.bank
                .transfer_from(
                    ctx.sender(),
                    self.id.to_payable(),
                    Coins { token_id, amount },
                    state,
                )
                .into_shielded_pool_err()?;
        }

        self.transact(
            proof,
            root,
            amount,
            commitment,
            nullifier,
            NoteKind::CreateAccount,
            memo,
            ctx,
            state,
        )?;

        self.has_shielded
            .set(ctx.sender(), &true, state)
            .into_shielded_pool_err()?;

        Ok(())
    }

    fn deposit(
        &mut self,
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        let token_id = self.get_token_id(state)?;
        self.bank
            .transfer_from(
                ctx.sender(),
                self.id.to_payable(),
                Coins { token_id, amount },
                state,
            )
            .into_shielded_pool_err()?;

        self.transact(
            proof,
            root,
            amount,
            commitment,
            nullifier,
            NoteKind::Deposit,
            memo,
            ctx,
            state,
        )?;

        Ok(())
    }

    fn withdraw(
        &mut self,
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        self.withdraw_to(
            proof,
            root,
            commitment,
            nullifier,
            amount,
            memo,
            ctx.sender(),
            ctx,
            state,
        )
    }

    pub fn withdraw_to(
        &mut self,
        proof: ProofBytes,
        root: HexHash,
        commitment: HexHash,
        nullifier: HexHash,
        amount: Amount,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
        to: impl Payable<S>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        self.transact(
            proof,
            root,
            amount,
            commitment,
            nullifier,
            NoteKind::Withdraw,
            memo,
            ctx,
            state,
        )?;

        let token_id = self.get_token_id(state)?;
        self.bank
            .transfer_from(self.id.to_payable(), to, Coins { token_id, amount }, state)
            .into_shielded_pool_err()?;

        Ok(())
    }

    fn transact(
        &mut self,
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        kind: NoteKind,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
        _ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        if self
            .nullifiers
            .get(&nullifier, state)
            .into_shielded_pool_err()?
            .unwrap_or(false)
        {
            return Err(ShieldedPoolError::DuplicateNullifier { nullifier });
        }

        if self
            .commitments
            .get(&commitment, state)
            .into_shielded_pool_err()?
            .unwrap_or(false)
        {
            return Err(ShieldedPoolError::DuplicateCommitment { commitment });
        }

        let mut tree = self
            .tree
            .get_or_err(state)
            .into_shielded_pool_err()?
            .into_shielded_pool_err()?;

        if !tree.is_known_root(root) {
            return Err(ShieldedPoolError::UnknownRoot { root });
        }

        Self::verify_proof(
            &proof,
            root,
            amount,
            nullifier,
            commitment,
            matches!(kind, NoteKind::CreateAccount),
            matches!(kind, NoteKind::CreateAccount | NoteKind::Deposit),
        )?;

        self.nullifiers
            .set(&nullifier, &true, state)
            .into_shielded_pool_err()?;
        self.commitments
            .set(&commitment, &true, state)
            .into_shielded_pool_err()?;

        tree.insert(commitment)
            .map_err(|e| ShieldedPoolError::Any(anyhow::anyhow!("{}", e)))?;
        self.tree.set(&tree, state).into_shielded_pool_err()?;

        self.emit_event(
            state,
            ShieldedPoolEvent::Note {
                kind,
                commitment,
                nullifier,
                amount: amount.0,
                memo: memo.as_ref().to_vec(),
            },
        );

        Ok(())
    }

    // ========================================================================
    // HELPERS
    // ========================================================================

    fn verify_proof(
        proof_bytes: &ProofBytes,
        root: HexHash,
        amount: Amount,
        nullifier: HexHash,
        commitment: HexHash,
        is_deposit: bool,
        is_new_account: bool,
    ) -> Result<(), ShieldedPoolError> {
        let proof = proof_bytes.to_ark_proof();
        let mut amount_bytes = HexHash::new([0u8; 32]);
        amount_bytes.0[16..].copy_from_slice(&amount.0.to_be_bytes());

        let (public_deposit_amount, public_withdraw_amount) = if is_deposit {
            (amount_bytes, HexHash::new([0; 32]))
        } else {
            (HexHash::new([0; 32]), amount_bytes)
        };

        let mut force_dummy_note = HexHash::new([0u8; 32]);
        if is_new_account {
            force_dummy_note.0[31] = 1;
        }

        let public_inputs = &PublicInputs(vec![
            root,
            nullifier,
            commitment,
            force_dummy_note,
            public_deposit_amount,
            public_withdraw_amount,
        ])
        .to_fr_vec();

        let valid = verifier::verify(&proof, public_inputs).map_err(|e| {
            ShieldedPoolError::VerificationFailed {
                message: e.to_string(),
            }
        })?;

        if valid {
            Ok(())
        } else {
            Err(ShieldedPoolError::InvalidProof)
        }
    }
}
