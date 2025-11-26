use axum::{Router, routing::put};

use crate::{app::AppState, storage::handlers};

pub fn storage_router(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{bucket}/{key}", put(handlers::put_object))
        .with_state(app_state)
}

#[cfg(test)]
pub fn storage_router_test(app_state: crate::app::TestAppState) -> Router {
    Router::new()
        .route("/{bucket}/{key}", put(handlers::put_object_test))
        .with_state(app_state)
}

#[cfg(test)]
mod tests {
    use axum_test::TestServer;

    use crate::{
        app::{MockAppStateOperations, TestAppState},
        storage::handlers::tests::build_multipart,
    };

    use super::*;

    #[tokio::test]
    async fn test_put_object() {
        let mut operations = MockAppStateOperations::new();
        operations.expect_upload().returning(|_, _, _| {
            Ok("https://test.s3.garage.aws.dxflrs.com/test/test.html".to_string())
        });
        let app_state = TestAppState::new(operations);
        let router = storage_router_test(app_state);

        const BYTES: &[u8] = "<!doctype html><html><body>Hello World</body></html>".as_bytes();

        let form = build_multipart(BYTES, "index.html", "text/html");

        let response = TestServer::new(router)
            .unwrap()
            .put("/beep/test")
            .multipart(form)
            .await;

        insta::assert_debug_snapshot!(response);
    }
}
