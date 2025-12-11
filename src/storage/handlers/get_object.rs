use axum::{
    body::Body,
    extract::{Path, State},
};
use http::Response;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
};

#[utoipa::path(
    get,
    path = "/{prefix}/{file_name}",
    tag = "storage",
    responses(
        (status = 200, description = "Upload successful", body = String),
        (status = 400, description = "Invalid request", body = String),
        (status = 500, description = "Internal server error", body = String),
    ),
)]
pub async fn get_object_handler(
    Path((prefix, file_name)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Response<Body>, ApiError> {
    get_object(format!("{}/{}", prefix, file_name), state).await
}

async fn get_object<S>(path: String, state: S) -> Result<Response<Body>, ApiError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let bucket = state.config().s3_bucket.clone();
    let (object, mime_type) = state.get_object(&bucket, &path).await.unwrap();
    let body = Body::from(object);
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", mime_type)
        .body(body)
        .unwrap())
}

#[cfg(test)]
pub async fn get_object_test(
    Path((prefix, file_name)): Path<(String, String)>,
    State(state): State<TestAppState>,
) -> Result<Response<Body>, ApiError> {
    get_object(format!("{}/{}", prefix, file_name), state).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Router, routing::get};
    use axum_test::TestServer;

    use crate::{
        app::{MockAppStateOperations, tests::TestAppState},
        config::Config,
    };

    use super::*;

    pub fn fake_router(app_state: TestAppState) -> Router {
        Router::new()
            .route("/{prefix}/{file_name}", get(get_object_test))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_get_object() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));
        operations
            .expect_get_object()
            .returning(|_, _| Ok((vec![1, 2, 3], "text/plain".to_string())));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/test-bucket/index.html")
            .await;
        insta::assert_debug_snapshot!(response);
    }
}
