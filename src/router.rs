use axum::{Router, routing::get};

use crate::{
    app::{AppState, AppStateOperations},
    error::CoreError,
    http::default_cors_layer,
    storage::router::storage_router,
};

pub async fn app(app_state: AppState) -> Result<Router, CoreError> {
    let config = app_state.config().unwrap();

    Ok(Router::new()
        .layer(default_cors_layer(&config.origins).unwrap())
        .merge(storage_router(app_state.clone()))
        .route("/status", get(|| async { "Alive !" }))
        .with_state(app_state))
}

#[cfg(test)]
pub async fn app_test(app_state: crate::app::TestAppState) -> Result<Router, CoreError> {
    use crate::storage::router::storage_router_test;

    let config = app_state.config().unwrap();

    Ok(Router::new()
        .layer(default_cors_layer(&config.origins).unwrap())
        .route("/status", get(|| async { "Alive !" }))
        .with_state(app_state.clone())
        .merge(storage_router_test(app_state.clone())))
}
