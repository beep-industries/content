use axum::{
    Router,
    routing::{get, post, put},
};

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{
    app::AppState,
    storage::handlers::{
        get_object::get_object_handler, post_object::post_sign_url_handler,
        put_object::put_object_handler,
    },
};

pub fn storage_router(app_state: AppState) -> Router {
    Router::new()
        .route("/{prefix}/{file_name}", put(put_object_handler))
        .route("/{prefix}/{file_name}", post(post_sign_url_handler))
        .route("/{prefix}/{file_name}", get(get_object_handler))
        .with_state(app_state)
}

#[cfg(test)]
pub fn storage_router_test(app_state: TestAppState) -> Router {
    use crate::storage::handlers::{
        get_object::get_object_test, post_object::post_sign_url_test, put_object::put_object_test,
    };

    Router::new()
        .route("/{prefix}/{file_name}", put(put_object_test))
        .route("/{prefix}/{file_name}", post(post_sign_url_test))
        .route("/{prefix}/{file_name}", get(get_object_test))
        .with_state(app_state)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum_test::TestServer;

    use crate::{
        app::MockAppStateOperations,
        config::Config,
        signed_url::{extractor::Claims, service::AvailableActions},
        storage::handlers::{post_object::SignUrlRequest, put_object::tests::build_multipart},
    };

    use super::*;

    #[tokio::test]
    async fn test_put_object() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Ok("Uploaded".to_string()));
        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));

        operations
            .expect_verify_parts()
            .returning(|_| Ok(Claims::default()));
        let app_state = TestAppState::new(operations);
        let router = storage_router_test(app_state);

        const BYTES: &[u8] = "<!doctype html><html><body>Hello World</body></html>".as_bytes();

        let form = build_multipart(BYTES, "index.html", "text/html");

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .put("/beep/index.html?action=Put&expires=1684969600&signature=test")
            .multipart(form)
            .await;

        insta::assert_debug_snapshot!(response);
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
        let router = storage_router_test(app_state);

        let payload = SignUrlRequest {
            action: AvailableActions::Put,
            expires_in_ms: 100,
        };
        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .post("/prefix/file_name")
            .json(&payload)
            .await;

        insta::assert_debug_snapshot!(response);
    }
}
