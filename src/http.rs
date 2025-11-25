use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::{config::Config, error::CoreError};

pub async fn serve(config: &Config) -> Result<(), CoreError> {
    let app = root_router(&config)?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .map_err(|e| CoreError::HttpServer(format!("{}", e)))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| CoreError::HttpServer(format!("{}", e)))
}

fn default_cors_layer(origins: &[String]) -> Result<CorsLayer, CoreError> {
    let origins = origins
        .iter()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .map_err(|e| CoreError::HttpServer(format!("{}", e)))
        })
        .collect::<Result<Vec<HeaderValue>, CoreError>>()?;

    Ok(CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_origin(origins))
}

fn root_router(config: &Config) -> Result<Router, CoreError> {
    Ok(Router::new()
        .layer(default_cors_layer(&config.origins)?)
        .route("/status", get(|| async { "Alive !" })))
}

#[cfg(test)]
mod tests {
    use axum::http::header;
    use axum_test::TestServer;

    use super::*;

    #[tokio::test]
    async fn test_default_cors_layer() {
        let config = Config {
            origins: vec!["https://beep.com".to_string()],
            port: 3000,
        };
        let service = root_router(&config).unwrap();
        let test_server = TestServer::new(service).unwrap();
        let response = test_server
            .get("/status")
            .add_header(header::ORIGIN, "https://beep.com")
            .await;
        insta::assert_debug_snapshot!(response.headers());
    }
}
