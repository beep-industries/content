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

    telemetry::run(config)
        .await
        .map_err(|e| CoreError::HttpServer(format!("Application error: {}", e)))
}

