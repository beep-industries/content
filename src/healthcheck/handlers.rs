use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    s3: bool,
}

pub async fn get_healthcheck_handler(
    State(state): State<AppState>,
) -> Result<Json<HealthCheck>, ApiError> {
    Ok(Json(healthcheck(state).await?))
}

#[cfg(test)]
pub async fn get_healthcheck_test(
    State(state): State<crate::app::TestAppState>,
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
