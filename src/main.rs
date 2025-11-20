use clap::Parser;
use dotenv::dotenv;


use crate::config::Config;
mod config;
mod error;
mod http;
mod app;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let config = Config::parse();

    app::run(config).await
}

