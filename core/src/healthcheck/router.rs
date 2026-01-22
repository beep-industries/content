use axum::{Router, routing::get};

#[cfg(test)]
use crate::app::tests::TestAppState;
use crate::{app::AppState, healthcheck::handlers};

pub fn healthcheck_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::get_healthcheck_handler))
        .with_state(app_state)
}

#[cfg(test)]
pub fn healthcheck_router_test(app_state: TestAppState) -> Router {
    Router::new()
        .route("/health", get(handlers::get_healthcheck_test))
        .with_state(app_state)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum_test::TestServer;

    use crate::{app::MockAppStateOperations, config::Config, s3::S3Error};

    use super::*;

    #[tokio::test]
    async fn test_mocked_probe() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Ok("Uploaded".to_string()));
        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));
        operations
            .expect_show_buckets()
            .returning(|| Ok(vec!["bucket1".to_string(), "bucket2".to_string()]));

        let app_state = TestAppState::new(operations);
        let router = healthcheck_router_test(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/health")
            .await;

        insta::assert_debug_snapshot!(response);
    }

    #[tokio::test]
    async fn test_mocked_probe_with_disconnected_s3() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _| Err(S3Error::NoBucketFound));
        operations
            .expect_config()
            .returning(|| Arc::new(Config::default()));
        operations
            .expect_show_buckets()
            .returning(|| Ok(vec!["bucket1".to_string(), "bucket2".to_string()]));

        let app_state = TestAppState::new(operations);
        let router = healthcheck_router_test(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/health")
            .await;
        insta::assert_debug_snapshot!(response);
    }
}
