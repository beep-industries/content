use clap::Parser;
use dotenv::dotenv;
use std::sync::Arc;

use crate::error::CoreError;

use crate::{app::AppState, config::Config, plumbing::create_service};

mod app;
mod config;
mod error;
mod http;
mod plumbing;
mod router;
mod s3;
mod storage;
mod telemetry;

#[cfg(test)]
mod router_test;

#[tokio::main]
async fn main() -> Result<(), CoreError> {
    dotenv().ok();
    let config = Config::parse();

    let guard = telemetry::init(&config)
        .map_err(|e| CoreError::HttpServer(format!("Telemetry error: {}", e)))?;

    let config = Arc::new(config);

    let content_service = Arc::new(create_service(config.clone()));
    let app_state: AppState = AppState::new(content_service, config.clone());
    let root = router::app(app_state).await.unwrap();

    let _ = crate::http::serve(root, config)
        .await
        .inspect_err(|e| eprintln!("{}", e));

    guard.shutdown().await;

    Ok(())
}
