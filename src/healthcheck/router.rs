use axum::{Router, routing::get};

use crate::{app::AppState, healthcheck::handlers};

pub fn healthcheck_router(app_state: AppState) -> Router {
    Router::new()
        .route("/status", get(handlers::get_healthcheck_handler))
        .with_state(app_state)
}

#[cfg(test)]
pub fn healthcheck_router_test(app_state: crate::app::TestAppState) -> Router {
    Router::new()
        .route("/status", get(handlers::get_healthcheck_test))
        .with_state(app_state)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum_test::TestServer;

    use crate::{
        app::{MockAppStateOperations, TestAppState},
        config::{Config, tests::bootstrap_integration_tests},
        plumbing::create_service,
        s3::{Garage, S3, S3Error},
    };

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

        let app_state = TestAppState::new(operations);
        let router = healthcheck_router_test(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/status")
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
        let app_state = TestAppState::new(operations);
        let router = healthcheck_router_test(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/status")
            .await;
        insta::assert_debug_snapshot!(response);
    }

    #[tokio::test]
    #[ignore]
    async fn test_probe_with_real_s3() {
        let config = Arc::new(bootstrap_integration_tests());
        let s3 = Garage::new(
            config.s3_endpoint.parse().expect("Invalid S3 endpoint"),
            &config.key_id,
            &config.secret_key,
        );
        let res = s3.show_buckets().await;
        assert!(res.is_ok());

        let content_service =
            Arc::new(create_service(config.clone()).expect("Service creation failed"));
        let app_state = AppState::new(content_service, config.clone());
        let router = healthcheck_router(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/status")
            .await;

        insta::assert_debug_snapshot!(response);
    }
}
