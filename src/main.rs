use clap::Parser;
use dotenv::dotenv;

use crate::config::Config;

mod config;
mod error;
mod http;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = Config::parse();
    let _ = crate::http::serve(config)
        .await
        .inspect_err(|e| eprintln!("{}", e));
}
