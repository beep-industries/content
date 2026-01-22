use axum::{body::Bytes, extract::State, http::HeaderMap, http::header::CONTENT_TYPE};
use utoipa::ToSchema;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
    signed_url::extractor::SignedUrl,
};

#[derive(ToSchema)]
#[allow(dead_code)]
struct UploadRequest {
    #[schema(content_media_type = "application/octet-stream")]
    pub file: Vec<u8>,
}

#[utoipa::path(
    put,
    path = "/{prefix}/{file_name}",
    tag = "storage",
    params(
        ("prefix" = String, Path, description = "Bucket prefix"),
        ("file_name" = String, Path, description = "File name"),
    ),
    request_body(content = UploadRequest, content_type = "application/octet-stream"),
    responses(
        (status = 200, description = "Upload successful", body = String),
        (status = 400, description = "Invalid request", body = String),
        (status = 500, description = "Internal server error", body = String),
    ),
)]
pub async fn put_object_handler(
    State(state): State<AppState>,
    SignedUrl(claims): SignedUrl,
    headers: HeaderMap,
    body: Bytes,
) -> Result<String, ApiError> {
    let (prefix, file_name) = claims.path;
    put_object(body, headers, state, prefix, file_name).await
}

#[cfg(test)]
pub async fn put_object_test(
    State(state): State<TestAppState>,
    SignedUrl(claims): SignedUrl,
    headers: HeaderMap,
    body: Bytes,
) -> Result<String, ApiError> {
    let (prefix, file_name) = claims.path;
    put_object(body, headers, state, prefix, file_name).await
}

/// Uploads a file from a raw binary request to S3.
/// The output of this method when successful is just a string "Uploaded"
/// confirming that the file was uploaded successfully.
///
/// # Examples
///
/// ```
/// let app_state = AppState::new(Arc::new(MockAppStateOperations::new()));
/// let router = Router::new()
///     .route("/{bucket}/{key}", put(put_object_handler))
///     .with_state(app_state);
/// ```
async fn put_object<S>(
    body: Bytes,
    headers: HeaderMap,
    state: S,
    prefix: String,
    file_name: String,
) -> Result<String, ApiError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let bucket = state.config().s3_bucket.clone();

    let key = format!("{}/{}", prefix, file_name);

    let file = state
        .guards()
        .check(&prefix, &key, body.to_vec(), content_type)
        .map_err(|e| e.into())?;

    state
        .upload(&bucket, &key, file)
        .await
        .map_err(|e| e.into())?;

    Ok("Uploaded".to_string())
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::{
        app::MockAppStateOperations,
        config::Config,
        guards::{FileType, Guard, GuardsBuilder},
        signed_url::{extractor::Claims, service::AvailableActions},
    };
    use axum::{Router, routing::put};
    use axum_test::TestServer;
    use reqwest::StatusCode;

    use super::*;

    pub fn fake_router(app_state: TestAppState) -> Router {
        Router::new()
            .route("/{bucket}/{key}", put(put_object_test))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_put_object() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Ok("Uploaded".to_string()));

        operations.expect_verify_parts().returning(|_| {
            Ok(Claims {
                path: ("test-bucket".to_string(), "index.html".to_string()),
                action: AvailableActions::Put,
            })
        });

        operations.expect_guards().returning(|| {
            Arc::new(
                GuardsBuilder::new()
                    .add("test-bucket", Guard::new(vec![FileType::Any]))
                    .build(),
            )
        });

        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        const BYTES: &[u8] = "<!doctype html><html><body>Hello World</body></html>".as_bytes();
        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        let client = TestServer::new(router).expect("Axum test server creation failed");
        let response = client
            .put("/test-bucket/index.html?action=Put&expires=1684969600&signature=test")
            .content_type(CONTENT_TYPE)
            .bytes(BYTES.into())
            .await;

        insta::assert_debug_snapshot!(response);
    }

    #[tokio::test]
    async fn test_put_object_empty_body() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Ok("Uploaded".to_string()));

        operations.expect_verify_parts().returning(|_| {
            Ok(Claims {
                path: ("test-bucket".to_string(), "index.html".to_string()),
                action: AvailableActions::Put,
            })
        });

        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));

        operations.expect_guards().returning(|| {
            Arc::new(
                GuardsBuilder::new()
                    .add("test-bucket", Guard::new(vec![FileType::Any]))
                    .build(),
            )
        });

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        let client = TestServer::new(router).expect("Axum test server creation failed");
        let response = client
            .put("/test-bucket/index.html?action=Put&expires=1684969600&signature=test")
            .content_type(CONTENT_TYPE)
            .bytes(vec![].into())
            .await;

        response.assert_status(StatusCode::OK);
    }
}
