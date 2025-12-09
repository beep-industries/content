use clap::Parser;
use dotenv::dotenv;
use std::sync::Arc;

use crate::error::CoreError;

use crate::signed_url::service::HMACUrlService;
use crate::signer::HMACSigner;
use crate::utils::get_time;
use crate::{app::AppState, config::Config, plumbing::create_service};

mod app;
mod config;
mod error;
mod healthcheck;
mod http;
mod plumbing;
mod router;
mod s3;
mod signed_url;
mod signer;
mod storage;
mod telemetry;
mod utils;

#[cfg(test)]
mod router_test;

#[tokio::main]
async fn main() -> Result<(), CoreError> {
    dotenv().ok();
    let config = Config::parse();
    let time = get_time();

    let telemetry_guard = telemetry::init(&config)
        .map_err(|e| CoreError::HttpServer(format!("Telemetry error: {}", e)))?;

    let config = Arc::new(config);

    let content_service =
        Arc::new(create_service(config.clone()).expect("Service creation failed"));

    let signer_service = Arc::new(
        HMACUrlService::new(
            HMACSigner::new(config.key_id.clone())
                .map_err(|e| CoreError::SigningKeyError(e.to_string()))?,
            time,
            "https://beep.com".to_string(),
        )
        .unwrap(),
    );
    let app_state: AppState = AppState::new(content_service, config.clone(), signer_service);
    let root = router::app(app_state)
        .await
        .expect("Router initialization error");

    let _ = crate::http::serve(root, config)
        .await
        .inspect_err(|e| eprintln!("{}", e));

    telemetry_guard.shutdown().await;

    Ok(())
}
