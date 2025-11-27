use axum::extract::{Multipart, Path, State};

use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
};

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
    State(state): State<crate::app::TestAppState>,
    multipart: Multipart,
) -> Result<String, ApiError> {
    put_object(multipart, state, prefix, file_name).await
}

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
        .unwrap();
    let chunk_data = field
        .bytes()
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?
        .to_vec();

    let bucket = state.config().unwrap().s3_bucket.clone();

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

    use crate::{
        app::{MockAppStateOperations, TestAppState},
        config::Config,
    };
    use axum::{Router, routing::put};
    use axum_test::{
        TestServer,
        multipart::{MultipartForm, Part},
    };

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
            .returning(|| Some(Arc::new(Config::default())));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        const BYTES: &[u8] = "<!doctype html><html><body>Hello World</body></html>".as_bytes();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        let form = build_multipart(BYTES, FILE_NAME, CONTENT_TYPE);

        let client = TestServer::new(router).unwrap();
        let response = client.put("/test-bucket/index.html").multipart(form).await;

        insta::assert_debug_snapshot!(response);
    }
}
