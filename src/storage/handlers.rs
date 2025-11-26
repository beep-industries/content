use axum::extract::{Multipart, Path, State};

use crate::{
    app::{AppState, AppStateOperations},
    error::ApiError,
};

pub async fn put_object(
    Path((bucket, key)): Path<(String, String)>,
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<String, ApiError> {
    put_zob_object(multipart, state, bucket, key).await
}

#[cfg(test)]
pub async fn put_object_test(
    Path((bucket, key)): Path<(String, String)>,
    State(state): State<crate::app::TestAppState>,
    multipart: Multipart,
) -> Result<String, ApiError> {
    put_zob_object(multipart, state, bucket, key).await
}

async fn put_zob_object<S>(
    mut multipart: Multipart,
    state: S,
    bucket: String,
    key: String,
) -> Result<String, ApiError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let mut file_name = String::new();
    #[allow(unused_variables)]
    let mut chunk_number = 0;
    #[allow(unused_variables)]
    let mut total_chunks = 0;
    let mut chunk_data = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "fileName" => file_name = field.text().await.unwrap().to_string(),
            #[allow(unused_assignments)]
            "chunkNumber" => chunk_number = field.text().await.unwrap().parse::<u32>().unwrap(),
            #[allow(unused_assignments)]
            "totalChunks" => total_chunks = field.text().await.unwrap().parse::<u32>().unwrap(),
            "chunk" => chunk_data = field.bytes().await.unwrap().to_vec(),
            _ => {}
        }
        state
            .upload(&bucket, &key, chunk_data.clone())
            .await
            .map_err(|e| e.into())?;
    }
    Ok(format!(
        "https://{}.s3.garage.aws.dxflrs.com/{}/{}",
        bucket, bucket, file_name
    ))
}

#[cfg(test)]
pub mod tests {
    use crate::app::{MockAppStateOperations, TestAppState};
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
        operations.expect_upload().returning(|_, _, _| {
            Ok("https://test.s3.garage.aws.dxflrs.com/test/test.html".to_string())
        });
        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        const BYTES: &[u8] = "<!doctype html><html><body>Hello World</body></html>".as_bytes();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html; charset=utf-8";

        let form = build_multipart(BYTES, FILE_NAME, CONTENT_TYPE);

        let client = TestServer::new(router).unwrap();
        let response = client.put("/test-bucket/test-key").multipart(form).await;

        insta::assert_debug_snapshot!(response);
    }
}
