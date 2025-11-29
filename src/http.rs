use std::sync::Arc;

use axum::{
    Router,
    http::{HeaderValue, Method},
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::{config::Config, error::CoreError};

pub async fn serve(app: Router, config: Arc<Config>) -> Result<(), CoreError> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .map_err(|e| CoreError::HttpServer(format!("{}", e)))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| CoreError::HttpServer(format!("{}", e)))
}

pub fn default_cors_layer(origins: &[String]) -> Result<CorsLayer, CoreError> {
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

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use axum::http::header;
    use axum_test::TestServer;

    use crate::{
        app::{MockAppStateOperations, tests::TestAppState},
        router,
    };

    use super::*;

    pub async fn test_server(config: Arc<Config>) -> TestServer {
        let mut mock = MockAppStateOperations::new();
        mock.expect_config().returning(move || config.clone());
        let app_state = TestAppState::new(mock);
        let service = router::app_test(app_state)
            .await
            .expect("Router creation failed");
        TestServer::new(service).expect("Test server creation failed")
    }

    #[tokio::test]
    async fn test_default_cors_layer() {
        let test_server = test_server(Arc::new(Config::default())).await;
        let response = test_server
            .get("/status")
            .add_header(header::ORIGIN, "https://beep.com")
            .await;
        insta::assert_debug_snapshot!(response.headers());
    }
}
