use crate::{
    app::AppStateOperations,
    signed_url::service::{AvailableActions, SignedUrlError},
};
use axum::extract::FromRequestParts;

#[derive(Debug, Clone, Default)]
pub struct Claims {
    pub action: AvailableActions,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct SignedUrl(pub Claims);

impl<S> FromRequestParts<S> for SignedUrl
where
    S: AppStateOperations + Send + Sync + 'static,
{
    type Rejection = SignedUrlError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Some(query) = parts.uri.query() else {
            return Err(SignedUrlError::MissingQueryParams(
                "Missing query params".to_string(),
            ));
        };

        let params: crate::signed_url::service::SignedURLParams = serde_qs::from_str(query)
            .map_err(|e| SignedUrlError::MissingQueryParams(e.to_string()))?;
        let action = params.action;
        let path = parts.uri.path().to_string();
        state.verify_parts(parts.clone())?;

        Ok(Self(Claims { action, path }))
    }
}

#[cfg(test)]
mod tests {
    use axum::{Router, routing::get};
    use axum_test::TestServer;
    use http::StatusCode;

    use super::*;
    async fn fake_handler(SignedUrl(claims): SignedUrl) -> String {
        format!("{:?}", claims)
    }

    #[tokio::test]
    async fn test_signed_url() {
        let mut operations = crate::app::MockAppStateOperations::new();
        operations
            .expect_verify_parts()
            .returning(|_| Ok(Claims::default()));
        let app_state = crate::app::tests::TestAppState::new(operations);
        let router: Router = Router::new()
            .route("/{path}/{file_name}", get(fake_handler))
            .with_state(app_state);

        let test_server = TestServer::new(router).expect("Test server creation failed");

        let response = test_server
            .get("/test-bucket/index.html?action=Put&expires=1684969600&signature=test")
            .await;

        // mocked verify_url should return Ok which means the signature is valid
        response.assert_status(StatusCode::OK);
    }

    #[tokio::test]
    async fn test_signed_url_invalid_signature() {
        let mut operations = crate::app::MockAppStateOperations::new();
        operations
            .expect_verify_parts()
            .returning(|_| Err(SignedUrlError::InvalidSignature));
        let app_state = crate::app::tests::TestAppState::new(operations);
        let router: Router = Router::new()
            .route("/{path}/{file_name}", get(fake_handler))
            .with_state(app_state);

        let test_server = TestServer::new(router).expect("Test server creation failed");

        let response = test_server
            .get("/test-bucket/index.html?action=Put&expires=1684969600&signature=test")
            .await;

        // mocked verify_url should return an error which means the signature is invalid
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}
