use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::{AppState, AppStateOperations},
    signed_url::service::{AvailableActions, SignedUrlError},
};

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SignUrlRequest {
    pub action: AvailableActions,
    pub expires_in_ms: u64,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SignUrlResponse {
    pub url: String,
}

fn post_sign_url<S>(
    path: String,
    request: SignUrlRequest,
    state: S,
) -> Result<SignUrlResponse, SignedUrlError>
where
    S: AppStateOperations + Send + Sync + 'static,
{
    let url = state.sign_url(path, request.action, request.expires_in_ms)?;

    Ok(SignUrlResponse { url })
}

#[utoipa::path(
    post,
    path = "/{prefix}/{file_name}",
    tag = "storage",
    request_body = SignUrlRequest,
    params(
        ("prefix" = String, Path, description = "Bucket prefix"),
        ("file_name" = String, Path, description = "File name"),
    ),
    responses(
        (status = 200, description = "Upload successful", body = SignUrlResponse),
        (status = 400, description = "Invalid request", body = String),
        (status = 500, description = "Internal server error", body = String),
    ),
)]
pub async fn post_sign_url_handler(
    Path((prefix, file_name)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(request): Json<SignUrlRequest>,
) -> Result<Json<SignUrlResponse>, SignedUrlError> {
    Ok(Json(post_sign_url(
        format!("{}/{}", prefix, file_name),
        request,
        state,
    )?))
}

#[cfg(test)]
pub async fn post_sign_url_test(
    Path((prefix, file_name)): Path<(String, String)>,
    State(state): State<TestAppState>,
    Json(request): Json<SignUrlRequest>,
) -> Result<Json<SignUrlResponse>, SignedUrlError> {
    Ok(Json(post_sign_url(
        format!("{}/{}", prefix, file_name),
        request,
        state,
    )?))
}

#[cfg(test)]
mod tests {
    use axum::{Router, routing::post};
    use axum_test::TestServer;

    use crate::{
        app::MockAppStateOperations,
        signed_url::{extractor::Claims, service::AvailableActions},
    };

    use super::*;

    pub fn fake_router(app_state: TestAppState) -> Router {
        Router::new()
            .route("/{prefix}/{file_name}", post(post_sign_url_test))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_post_sign_url() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_sign_url()
            .returning(|_, _, _| Ok("https://beep.com/prefix/file_name".to_string()));
        operations
            .expect_verify_parts()
            .returning(|_| Ok(Claims::default()));

        let app_state = TestAppState::new(operations);
        let router = fake_router(app_state);

        let client = TestServer::new(router).expect("Axum test server creation failed");
        let payload = SignUrlRequest {
            action: AvailableActions::Put,
            expires_in_ms: 100,
        };
        let response = client.post("/prefix/file_name").json(&payload).await;
        insta::assert_debug_snapshot!(response);
    }
}
