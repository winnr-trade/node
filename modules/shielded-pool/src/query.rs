use crate::ShieldedPoolModule;
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{
    axum::{extract::Query, routing::get, Json, Router},
    UnwrapInfallible,
};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HasShieldedParams<S: Spec> {
    user_address: S::Address,
}

#[derive(Debug, serde::Serialize)]
struct HasShieldedResponse {
    has_shielded: bool,
}

impl<S: Spec> ShieldedPoolModule<S> {
    async fn route_has_shielded(
        params: Query<HasShieldedParams<S>>,
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
    ) -> Json<HasShieldedResponse> {
        let has_shielded = state
            .has_shielded
            .get(&params.user_address, &mut acc)
            .unwrap_infallible()
            .unwrap_or(false);

        Json(HasShieldedResponse { has_shielded })
    }
}

impl<S: Spec> HasCustomRestApi for ShieldedPoolModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route("/has-shielded", get(Self::route_has_shielded))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
