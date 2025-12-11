use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthCheck {
    s3: bool,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "healthcheck",
    responses(
        (status = 200, description = "Health check successful", body = HealthCheck),
        (status = 500, description = "Internal server error", body = ApiError)
    )
)]
pub async fn get_healthcheck_handler(
    State(state): State<AppState>,
) -> Result<Json<HealthCheck>, ApiError> {
    Ok(Json(healthcheck(state).await?))
}

#[cfg(test)]
pub async fn get_healthcheck_test(
    State(state): State<TestAppState>,
) -> Result<Json<HealthCheck>, ApiError> {
    Ok(Json(healthcheck(state).await?))
}

async fn healthcheck<S>(state: S) -> Result<HealthCheck, ApiError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let s3 = state.show_buckets().await.is_ok();
    Ok(HealthCheck { s3 })
}
