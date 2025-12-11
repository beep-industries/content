use axum::extract::{Multipart, Path, State};
use utoipa::ToSchema;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
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
    request_body(content = UploadRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Upload successful", body = String),
        (status = 400, description = "Invalid request", body = String),
        (status = 500, description = "Internal server error", body = String),
    ),
)]
pub async fn put_object_handler(
    Path((prefix, file_name)): Path<(String, String)>,
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<String, ApiError> {
    put_object(multipart, state, prefix, file_name).await
}

#[cfg(test)]
pub async fn put_object_test(
    Path((prefix, file_name)): Path<(String, String)>,
    State(state): State<TestAppState>,
    multipart: Multipart,
) -> Result<String, ApiError> {
    put_object(multipart, state, prefix, file_name).await
}

/// Uploads a file from a multipart request to S3.
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
    mut multipart: Multipart,
    state: S,
    prefix: String,
    file_name: String,
) -> Result<String, ApiError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let field = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("Empty request".to_string()))?
        .ok_or_else(|| ApiError::BadRequest("No file".to_string()))?;

    let chunk_data = field
        .bytes()
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?
        .to_vec();

    let bucket = state.config().s3_bucket.clone();

    let key = format!("{}/{}", prefix, file_name);

    state
        .upload(&bucket, &key, chunk_data.clone())
        .await
        .map_err(|e| e.into())?;

    Ok("Uploaded".to_string())
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::{app::MockAppStateOperations, config::Config};
    use axum::{Router, routing::put};
    use axum_test::{
        TestServer,
        multipart::{MultipartForm, Part},
    };
    use reqwest::StatusCode;

    use super::*;

    pub fn build_multipart(
        content: &'static [u8],
        file_name: &str,
        content_type: &str,
    ) -> MultipartForm {
        let part = Part::bytes(content)
            .file_name(file_name)
            .mime_type(content_type);

        MultipartForm::new().add_part("file", part)
    }

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

        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        const BYTES: &[u8] = "<!doctype html><html><body>Hello World</body></html>".as_bytes();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        let form = build_multipart(BYTES, FILE_NAME, CONTENT_TYPE);

        let client = TestServer::new(router).expect("Axum test server creation failed");
        let response = client.put("/test-bucket/index.html").multipart(form).await;

        insta::assert_debug_snapshot!(response);
    }

    #[tokio::test]
    async fn test_put_object_empty_request() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Ok("Uploaded".to_string()));

        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        let client = TestServer::new(router).expect("Axum test server creation failed");
        let response = client.put("/test-bucket/index.html").await;

        response.assert_status(StatusCode::BAD_REQUEST);
        response.assert_text("Invalid `boundary` for `multipart/form-data` request");
    }

    #[tokio::test]
    async fn test_put_object_empty_part() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Ok("Uploaded".to_string()));

        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        const BYTES: &[u8] = "".as_bytes();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        let form = build_multipart(BYTES, FILE_NAME, CONTENT_TYPE);

        let client = TestServer::new(router).expect("Axum test server creation failed");
        let response = client.put("/test-bucket/index.html").multipart(form).await;

        response.assert_status(StatusCode::OK);
    }
}
