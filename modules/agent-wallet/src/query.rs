use crate::types::{AgentPolicy, OwnerAgentKey};
use crate::AgentWalletModule;
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{
    axum::{routing::get, Json, Router},
    UnwrapInfallible,
};
use sov_modules_api::rest::utils::{errors, ApiResult, Query};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct PolicyQueryParams<S: Spec> {
    owner: S::Address,
    agent: S::Address,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct AgentOwnerQueryParams<S: Spec> {
    agent: S::Address,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct NonceQueryParams<S: Spec> {
    owner: S::Address,
}

impl<S: Spec> AgentWalletModule<S> {
    /// GET /agent-wallet/policy
    ///
    /// Returns the policy for the given (owner, agent) pair, if it exists.
    async fn route_get_policy(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        params: Query<PolicyQueryParams<S>>,
    ) -> ApiResult<AgentPolicy> {
        let key = OwnerAgentKey {
            owner: params.owner,
            agent: params.agent,
        };
        let policy = state
            .policies
            .get(&key, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| {
                errors::not_found_404(
                    "AgentPolicy",
                    format!("owner: {}, agent: {}", params.owner, params.agent),
                )
            })?;

        Ok(Json(policy))
    }

    /// GET /agent-wallet/owner
    ///
    /// Returns the owner address that registered the given agent, if any.
    async fn route_get_owner(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        params: Query<AgentOwnerQueryParams<S>>,
    ) -> ApiResult<S::Address> {
        let owner = state
            .agent_to_owner
            .get(&params.agent, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| errors::not_found_404("Agent", params.agent.to_string()))?;

        Ok(Json(owner))
    }

    /// GET /agent-wallet/nonce
    ///
    /// Returns the current nonce for the given owner address
    async fn route_get_owner_nonce(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        params: Query<NonceQueryParams<S>>,
    ) -> ApiResult<u64> {
        let nonce = state
            .owner_nonces
            .get(&params.owner, &mut acc)
            .unwrap_infallible()
            // Default nonce is 0 if owner has never registered an agent
            .unwrap_or(0);

        Ok(Json(nonce))
    }
}

impl<S: Spec> HasCustomRestApi for AgentWalletModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route("/policy", get(Self::route_get_policy))
            .route("/owner", get(Self::route_get_owner))
            .route("/nonce", get(Self::route_get_owner_nonce))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
