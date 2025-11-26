use clap::Parser;
use dotenv::dotenv;


use crate::config::Config;
mod config;
mod error;
mod http;
mod telemetry;

use crate::error::CoreError;



#[tokio::main]
async fn main() -> Result<(), CoreError>{
    dotenv().ok();
    let config = Config::parse();

    let guard = telemetry::init(&config)
        .map_err(|e| CoreError::HttpServer(format!("Telemetry error: {}", e)))?;

    let _ = crate::http::serve(&config)
        .await
        .inspect_err(|e| eprintln!("{}", e));

    guard.shutdown().await;

    Ok(())
}

