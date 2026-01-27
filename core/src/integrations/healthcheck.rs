use std::sync::Arc;

use axum_test::TestServer;

use crate::{
    app::AppState,
    config::tests::bootstrap_integration_tests,
    guards::{FileType, Guard, GuardsBuilder},
    healthcheck::router::healthcheck_router,
    plumbing::create_service,
    prefixes::Prefix,
    s3::{Garage, S3},
    signed_url::service::HMACUrlService,
    signer::HMACSigner,
    utils::get_time,
};

#[tokio::test]
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
    let guards = Arc::new(
        GuardsBuilder::new()
            .add(
                Prefix::ProfilePicture,
                Guard::new(vec![FileType::ImageJPEG]),
            )
            .build(),
    );
    let app_state = AppState::new(content_service, config.clone(), signer_service, guards);
    let router = healthcheck_router(app_state);

    let response = TestServer::new(router)
        .expect("Axum test server creation failed")
        .get("/health")
        .await;

    insta::assert_debug_snapshot!(response);
}
