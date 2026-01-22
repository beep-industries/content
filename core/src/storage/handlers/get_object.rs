use axum::{body::Body, extract::State};
use http::Response;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
    signed_url::extractor::SignedUrl,
};

#[utoipa::path(
    get,
    path = "/{prefix}/{file_name}",
    tag = "storage",
    params(
        ("prefix" = String, Path, description = "Bucket prefix"),
        ("file_name" = String, Path, description = "File name"),
    ),
    responses(
        (status = 200, description = "Upload successful", body = String),
        (status = 400, description = "Invalid request", body = String),
        (status = 500, description = "Internal server error", body = String),
    ),
)]
pub async fn get_object_handler(
    State(state): State<AppState>,
    SignedUrl(claims): SignedUrl,
) -> Result<Response<Body>, ApiError> {
    let (prefix, file_name) = claims.path;
    get_object(format!("{}/{}", prefix, file_name), state).await
}

async fn get_object<S>(path: String, state: S) -> Result<Response<Body>, ApiError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let bucket = state.config().s3_bucket.clone();
    let (object, mime_type) = state
        .get_object(&bucket, &path)
        .await
        .map_err(|e| e.into())?;
    let body = Body::from(object);
    Response::builder()
        .status(200)
        .header("Content-Type", mime_type)
        .body(body)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))
}

#[cfg(test)]
pub async fn get_object_test(
    SignedUrl(claims): SignedUrl,
    State(state): State<TestAppState>,
) -> Result<Response<Body>, ApiError> {
    let (prefix, file_name) = claims.path;
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
        signed_url::{extractor::Claims, service::AvailableActions},
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
        operations.expect_verify_parts().returning(|_| {
            Ok(Claims {
                path: ("test-bucket".to_string(), "index.html".to_string()),
                action: AvailableActions::Put,
            })
        });

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/test-bucket/index.html")
            .await;
        insta::assert_debug_snapshot!(response);
    }
}
