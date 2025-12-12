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

    use crate::{
        app::MockAppStateOperations,
        config::{Config, tests::bootstrap_integration_tests},
        plumbing::create_service,
        s3::{Garage, S3, S3Error},
        signed_url::service::HMACUrlService,
        signer::HMACSigner,
        utils::get_time,
    };

    use super::*;

    #[tokio::test]
    async fn test_mocked_probe() {
        let mut operations = MockAppStateOperations::new();
        operations
            .expect_upload()
            .returning(|_, _, _, _| Ok("Uploaded".to_string()));
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
            .returning(|_, _, _, _| Err(S3Error::NoBucketFound));
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
        let signer_service = Arc::new(
            HMACUrlService::new(
                HMACSigner::new(config.key_id.clone()).expect("Invalid signing key"),
                get_time(),
                "https://beep.com".to_string(),
            )
            .expect("Invalid signing key"),
        );
        let app_state = AppState::new(content_service, config.clone(), signer_service);
        let router = healthcheck_router(app_state);

        let response = TestServer::new(router)
            .expect("Axum test server creation failed")
            .get("/health")
            .await;

        insta::assert_debug_snapshot!(response);
    }
}
