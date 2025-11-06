use axum::{
    Router,
    http::{HeaderValue, Method, header::InvalidHeaderValue},
    routing::get,
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::config::Config;

pub async fn serve(config: Config) -> Result<(), HttpInfrastructureError> {
    let app = root_router(&config)?;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .map_err(HttpInfrastructureError::TcpListener)?;
    axum::serve(listener, app)
        .await
        .map_err(HttpInfrastructureError::AxumServe)
}

fn default_cors_layer(origins: &Vec<String>) -> Result<CorsLayer, HttpInfrastructureError> {
    println!("Origins: {:?}", origins);
    let origins = origins
        .iter()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .map_err(HttpInfrastructureError::Origins)
        })
        .collect::<Result<Vec<HeaderValue>, HttpInfrastructureError>>()?;

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

fn root_router(config: &Config) -> Result<Router, HttpInfrastructureError> {
    Ok(Router::new()
        .layer(default_cors_layer(&config.origins)?)
        .route("/status", get(|| async { "Alive !" })))
}

#[derive(Debug)]
pub enum HttpInfrastructureError {
    TcpListener(std::io::Error),
    AxumServe(std::io::Error),
    Origins(InvalidHeaderValue),
}

impl HttpInfrastructureError {
    pub fn handle(&self) {
        match self {
            HttpInfrastructureError::TcpListener(e) => {
                eprintln!("TcpListenerError: {}", e);
            }
            HttpInfrastructureError::AxumServe(e) => {
                eprintln!("AxumServeError: {}", e);
            }
            HttpInfrastructureError::Origins(e) => {
                eprintln!("OriginsError: {}", e);
            }
        }
    }
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
