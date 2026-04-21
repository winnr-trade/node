use crate::types::{AgentPolicy, OwnerAgentKey};
use crate::AgentWalletModule;
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{
    axum::{routing::get, Json, Router},
    UnwrapInfallible,
};
use sov_modules_api::rest::utils::{errors, ApiResult, Path};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};

impl<S: Spec> AgentWalletModule<S> {
    /// GET /agent-wallet/policy/:owner/:agent
    ///
    /// Returns the policy for the given (owner, agent) pair, if it exists.
    async fn route_get_policy(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        Path((owner, agent)): Path<(S::Address, S::Address)>,
    ) -> ApiResult<AgentPolicy> {
        let key = OwnerAgentKey {
            owner: owner.clone(),
            agent: agent.clone(),
        };
        let policy = state
            .policies
            .get(&key, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| {
                errors::not_found_404(
                    "AgentPolicy",
                    format!("owner={owner}, agent={agent}"),
                )
            })?;

        Ok(Json(policy))
    }

    /// GET /agent-wallet/owner/:agent
    ///
    /// Returns the owner address that registered the given agent, if any.
    async fn route_get_owner(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        Path(agent): Path<S::Address>,
    ) -> ApiResult<S::Address> {
        let owner = state
            .agent_to_owner
            .get(&agent, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| errors::not_found_404("Agent", agent.to_string()))?;

        Ok(Json(owner))
    }
}

impl<S: Spec> HasCustomRestApi for AgentWalletModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route(
                "/agent-wallet/policy/:owner/:agent",
                get(Self::route_get_policy),
            )
            .route("/agent-wallet/owner/:agent", get(Self::route_get_owner))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
