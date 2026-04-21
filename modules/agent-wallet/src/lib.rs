//! Agent Wallet Module
//!
//! Allows owners to delegate scoped trading permissions to agent wallets.
//! An agent wallet is a locally-generated ed25519 keypair that can place,
//! update, or cancel orders on behalf of the owning main wallet.
//!
//! ## Flow
//! 1. Owner signs a `RegisterAgent` transaction with Phantom.
//! 2. Agent signs normal rollup transactions with its local keypair.
//! 3. The orderbook calls `resolve_principal()` to obtain the effective owner.

mod call;
mod error;
mod event;
mod genesis;
pub mod types;

#[cfg(feature = "native")]
mod query;

pub use call::CallMessage;
pub use error::{AgentWalletError, IntoAgentWalletError};
pub use event::Event;
pub use genesis::AgentWalletGenesisConfig;
pub use types::{
    AgentPolicy, OwnerAgentKey, SCOPE_ALL_VALID, SCOPE_CANCEL_ALL_ORDERS, SCOPE_CANCEL_ORDER,
    SCOPE_PLACE_ORDER,
};

use sov_chain_state::ChainState;
use sov_modules_api::{
    Context, EventEmitter, GenesisState, Module, ModuleId, ModuleInfo, ModuleRestApi, Spec,
    StateMap, TxState,
};

/// Agent Wallet Module — scoped delegation of trading permissions.
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct AgentWalletModule<S: Spec> {
    /// Module identifier.
    #[id]
    pub id: ModuleId,

    /// Delegation policies indexed by (owner, agent).
    #[state]
    pub policies: StateMap<OwnerAgentKey<S>, AgentPolicy>,

    /// Reverse index: agent address → owner address.
    ///
    /// Enables O(1) owner lookup in `resolve_principal` without scanning.
    #[state]
    pub agent_to_owner: StateMap<S::Address, S::Address>,

    /// Chain state for reading the current block time (expiry checks).
    #[module]
    pub chain_state: ChainState<S>,
}

impl<S: Spec> Module for AgentWalletModule<S> {
    type Spec = S;
    type Config = AgentWalletGenesisConfig<S>;
    type CallMessage = CallMessage<S>;
    type Event = Event;
    type Error = AgentWalletError;

    fn genesis(
        &mut self,
        header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.init_module(header, config, state)
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), Self::Error> {
        match msg {
            CallMessage::RegisterAgent {
                agent,
                scopes,
                expires_at,
            } => self.register_agent(agent, scopes, expires_at, ctx, state),

            CallMessage::RevokeAgent { agent } => self.revoke_agent(agent, ctx, state),
        }
    }
}

// ============================================================================
// Call handlers + public authorization API
// ============================================================================

impl<S: Spec> AgentWalletModule<S> {
    /// Register (or replace) an agent delegation.
    ///
    /// `ctx.sender()` is the owner.
    fn register_agent(
        &mut self,
        agent: S::Address,
        scopes: u32,
        expires_at: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), AgentWalletError> {
        let owner = ctx.sender();

        // Validate: agent must differ from owner.
        if *owner == agent {
            return Err(AgentWalletError::AgentCannotBeOwner);
        }

        // Validate: scopes must be non-zero and only contain known bits.
        if scopes == 0 || scopes & !SCOPE_ALL_VALID != 0 {
            return Err(AgentWalletError::InvalidScopes);
        }

        // Validate: expires_at must be 0 (never) or strictly in the future.
        if expires_at != 0 {
            let now = self
                .chain_state
                .get_time(state)
                .into_agent_wallet_err()?
                .as_millis() as u64;
            if expires_at <= now {
                return Err(AgentWalletError::InvalidExpiry);
            }
        }

        let key = OwnerAgentKey {
            owner: owner.clone(),
            agent: agent.clone(),
        };
        let policy = AgentPolicy { scopes, expires_at };

        self.policies
            .set(&key, &policy, state)
            .into_agent_wallet_err()?;

        self.agent_to_owner
            .set(&agent, owner, state)
            .into_agent_wallet_err()?;

        self.emit_event(
            state,
            Event::AgentRegistered {
                owner: owner.to_string(),
                agent: agent.to_string(),
                scopes,
                expires_at,
            },
        );

        Ok(())
    }

    /// Revoke an agent delegation.
    ///
    /// `ctx.sender()` must be the owner who registered this agent.
    fn revoke_agent(
        &mut self,
        agent: S::Address,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), AgentWalletError> {
        let owner = ctx.sender();

        // Confirm this agent is delegated to the calling owner.
        let stored_owner = self
            .agent_to_owner
            .get(&agent, state)
            .into_agent_wallet_err()?
            .ok_or_else(|| AgentWalletError::AgentNotDelegated {
                agent: agent.to_string(),
            })?;

        if stored_owner != *owner {
            return Err(AgentWalletError::Unauthorized {
                action: "revoke agent registered by another owner".to_string(),
            });
        }

        let key = OwnerAgentKey {
            owner: owner.clone(),
            agent: agent.clone(),
        };

        self.policies.remove(&key, state).into_agent_wallet_err()?;
        self.agent_to_owner
            .remove(&agent, state)
            .into_agent_wallet_err()?;

        self.emit_event(
            state,
            Event::AgentRevoked {
                owner: owner.to_string(),
                agent: agent.to_string(),
            },
        );

        Ok(())
    }

    /// Check whether `agent` is authorized for `required_scope` and return the
    /// effective owner (principal) address if so.
    ///
    /// Called by the orderbook in Phase 2 to resolve the acting principal before
    /// attributing an order. Returns `AgentWalletError` on any failure.
    pub fn resolve_principal(
        &self,
        agent: &S::Address,
        required_scope: u32,
        state: &mut impl TxState<S>,
    ) -> Result<S::Address, AgentWalletError> {
        // 1. Reverse lookup: agent → owner.
        let owner = self
            .agent_to_owner
            .get(agent, state)
            .into_agent_wallet_err()?
            .ok_or_else(|| AgentWalletError::AgentNotDelegated {
                agent: agent.to_string(),
            })?;

        // 2. Load policy.
        let key = OwnerAgentKey {
            owner: owner.clone(),
            agent: agent.clone(),
        };
        let policy = self
            .policies
            .get(&key, state)
            .into_agent_wallet_err()?
            .ok_or_else(|| AgentWalletError::PolicyNotFound {
                agent: agent.to_string(),
            })?;

        // 3. Check expiry (0 = never expires).
        if policy.expires_at != 0 {
            let now = self
                .chain_state
                .get_time(state)
                .into_agent_wallet_err()?
                .as_millis() as u64;
            if now >= policy.expires_at {
                return Err(AgentWalletError::PolicyExpired {
                    agent: agent.to_string(),
                    expired_at: policy.expires_at,
                });
            }
        }

        // 4. Check scope.
        if policy.scopes & required_scope != required_scope {
            return Err(AgentWalletError::ScopeNotGranted {
                required: required_scope,
                granted: policy.scopes,
            });
        }

        Ok(owner)
    }

    /// Like `resolve_principal`, but returns the sender address unchanged when
    /// it has no delegation entry — i.e. the sender is acting as their own
    /// principal (normal direct-user flow).
    ///
    /// Pass-through behaviour means existing calls from users without an agent
    /// setup continue to work without any changes on the caller side.
    pub fn resolve_principal_or_self(
        &self,
        sender: &S::Address,
        required_scope: u32,
        state: &mut impl TxState<S>,
    ) -> Result<S::Address, AgentWalletError> {
        match self
            .agent_to_owner
            .get(sender, state)
            .into_agent_wallet_err()?
        {
            None => Ok(sender.clone()),
            Some(_) => self.resolve_principal(sender, required_scope, state),
        }
    }
}
