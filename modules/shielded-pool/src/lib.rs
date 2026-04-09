//! Shielded pool module for private balance management.
//!
//! This module enables users to deposit tokens into a shielded pool and withdraw
//! them privately using zero-knowledge proofs. Deposited tokens are held by the
//! pool and tracked via commitments in an incremental Merkle tree. Withdrawals
//! require proof of a valid commitment and consume a nullifier to prevent
//! double-spending.
//!
//! **Status: Early scaffold — several components are placeholder implementations.**
//! See inline `TODO`/`FIXME` comments for known gaps.

mod call;
mod error;
mod event;
mod genesis;
mod hash;
mod tree;
pub mod verifier;

use std::iter;

pub use call::{CallMessage, Proof};
pub use error::ShieldedPoolError;
pub use event::ShieldedPoolEvent;
pub use genesis::ShieldedPoolGenesisConfig;

use crate::{error::IntoShieldedPoolError, hash::poseidon_t4, tree::IncrementalMerkleTree};
use sov_bank::{Amount, Bank, Coins, IntoPayable, Payable, TokenId};
use sov_chain_state::ChainState;
use sov_modules_api::{
    self, Context, HexHash, Module, ModuleId, ModuleInfo, ModuleRestApi, SafeVec, Spec, StateMap,
    StateValue, TxState,
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

    /// Allowlist of token IDs accepted by the pool.
    // TODO: Enforce this check in deposit/withdraw handlers.
    #[state]
    pub supported_tokens: StateMap<TokenId, bool>,

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
            CallMessage::ShieldFirst {
                token_id,
                amount,
                blinded_address,
            } => self.deposit_first(token_id, amount, blinded_address, ctx, state),

            CallMessage::Shield {
                proof,
                token_id,
                amount,
                commitment,
                nullifier,
            } => self.deposit(proof, token_id, amount, commitment, nullifier, ctx, state),

            CallMessage::UnShield {
                proof,
                token_id,
                amount,
                commitment,
                nullifier,
            } => self.withdraw(proof, token_id, amount, commitment, nullifier, ctx, state),
        }
    }
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

impl<S: Spec> ShieldedPoolModule<S> {
    fn deposit_first(
        &mut self,
        token_id: TokenId,
        amount: u64,
        blinded_address: HexHash,
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

        self.bank
            .transfer_from(
                ctx.sender(),
                self.id.to_payable(),
                Coins {
                    token_id,
                    amount: Amount(amount as u128),
                },
                state,
            )
            .into_shielded_pool_err()?;

        let commitment = Self::calculate_commitment(token_id, amount, blinded_address);

        self.tree
            .get_or_err(state)
            .into_shielded_pool_err()?
            .into_shielded_pool_err()?
            .insert(commitment)
            .into_shielded_pool_err()?;

        self.has_shielded
            .set(ctx.sender(), &true, state)
            .into_shielded_pool_err()?;

        self.commitments
            .set(&commitment, &true, state)
            .into_shielded_pool_err()?;

        Ok(())
    }

    fn deposit(
        &mut self,
        proof: Proof,
        token_id: TokenId,
        amount: u64,
        commitment: HexHash,
        nullifier: HexHash,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        self.transact(proof, token_id, amount, commitment, nullifier, ctx, state)?;

        self.bank
            .transfer_from(
                ctx.sender(),
                self.id.to_payable(),
                Coins {
                    token_id,
                    amount: Amount(amount as u128),
                },
                state,
            )
            .into_shielded_pool_err()?;

        Ok(())
    }

    fn withdraw(
        &mut self,
        proof: Proof,
        token_id: TokenId,
        amount: u64,
        commitment: HexHash,
        nullifier: HexHash,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        self.withdraw_to(
            proof,
            commitment,
            nullifier,
            token_id,
            amount,
            ctx.sender(),
            ctx,
            state,
        )
    }

    pub fn withdraw_to(
        &mut self,
        proof: Proof,
        commitment: HexHash,
        nullifier: HexHash,
        token_id: TokenId,
        amount: u64,
        to: impl Payable<S>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        self.transact(proof, token_id, amount, commitment, nullifier, ctx, state)?;

        self.bank
            .transfer_from(
                self.id.to_payable(),
                to,
                Coins {
                    token_id,
                    amount: Amount(amount as u128),
                },
                state,
            )
            .into_shielded_pool_err()?;

        Ok(())
    }

    fn transact(
        &mut self,
        _proof: Proof,
        _token_id: TokenId,
        _amount: u64,
        commitment: HexHash,
        nullifier: HexHash,
        _ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), ShieldedPoolError> {
        // Check if the nullifier has already been used
        if self
            .nullifiers
            .get(&nullifier, state)
            .into_shielded_pool_err()?
            .unwrap_or(false)
        {
            return Err(ShieldedPoolError::DuplicateNullifier { nullifier });
        }

        // Check if the commitment has already been used
        if self
            .commitments
            .get(&commitment, state)
            .into_shielded_pool_err()?
            .unwrap_or(false)
        {
            return Err(ShieldedPoolError::DuplicateCommitment { commitment });
        }

        // TODO: Verify the ZK proof before proceeding.
        // let is_valid = verify_proof(proof, public_values_bytes, sp1_vkey_hash);
        let is_valid = true; // Placeholder until proof verification is implemented
        if !is_valid {
            return Err(ShieldedPoolError::InvalidProof);
        }

        // Mark the nullifier and commitment as used to prevent double-spending and duplicates.
        self.nullifiers
            .set(&nullifier, &true, state)
            .into_shielded_pool_err()?;
        self.commitments
            .set(&commitment, &true, state)
            .into_shielded_pool_err()?;

        // Insert the new commitment into the tree.
        self.tree
            .get_or_err(state)
            .into_shielded_pool_err()?
            .into_shielded_pool_err()?
            .insert(commitment)
            .into_shielded_pool_err()?;

        Ok(())
    }

    // ========================================================================
    // HELPERS
    // ========================================================================
    // fn shielded_pool_address(&self) -> Payable<S> {
    //     self.id().to_payable()
    // const SHIELDED_POOL_ADDRESS: [u8; 32] = [2; 32];
    // S::Address::from(CredentialId::from_bytes(SHIELDED_POOL_ADDRESS))
    // }

    /// Compute a binding commitment from the deposit parameters.
    fn calculate_commitment(token_id: TokenId, amount: u64, blinded_address: HexHash) -> HexHash {
        let h = poseidon_t4(
            &token_id.as_bytes(),
            &iter::repeat(0u8)
                .take(24)
                .chain(amount.to_be_bytes())
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap(),
            &blinded_address.0,
        );

        HexHash::new(h)
    }
}
