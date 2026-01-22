use clap::Parser;
use content_core::{config::Config, error::CoreError, telemetry, utils::get_time};
use dotenv::dotenv;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), CoreError> {
    dotenv().ok();
    let config = Config::parse();
    let time = get_time();

    let telemetry_guard = telemetry::init(&config)
        .map_err(|e| CoreError::HttpServer(format!("Telemetry error: {}", e)))?;

    let config = Arc::new(config);

    content_core::app(config, time).await?;

    telemetry_guard.shutdown().await;

    Ok(())
}
