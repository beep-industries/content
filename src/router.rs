use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::openapi::ApiDoc;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    error::CoreError,
    healthcheck::router::healthcheck_router,
    http::default_cors_layer,
    storage::router::storage_router,
};

pub async fn app(app_state: AppState) -> Result<Router, CoreError> {
    let config = app_state.clone().config();
    let openapi = ApiDoc::openapi();

    Ok(Router::new()
        .layer(default_cors_layer(&config.origins)?)
        .merge(Scalar::with_url("/docs", openapi.clone()))
        .merge(healthcheck_router(app_state.clone()))
        .merge(storage_router(app_state.clone())))
}

#[cfg(test)]
pub async fn app_test(app_state: TestAppState) -> Result<Router, CoreError> {
    use crate::{
        healthcheck::router::healthcheck_router_test, storage::router::storage_router_test,
    };

    let config = app_state.config();

    Ok(Router::new()
        .layer(default_cors_layer(&config.origins)?)
        .merge(healthcheck_router_test(app_state.clone()))
        .merge(storage_router_test(app_state.clone())))
}
