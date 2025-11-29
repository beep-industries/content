#[cfg(test)]
mod tests {
    use crate::{
        app::{MockAppStateOperations, TestAppState},
        config::Config,
        router::app_test,
    };
    use axum_test::TestServer;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_status_endpoint() {
        let mut mock = MockAppStateOperations::new();

        mock.expect_config().returning(|| {
            Arc::new(Config {
                origins: vec!["http://localhost:3000".to_string()],
                ..Default::default()
            })
        });

        let test_state = TestAppState::new(mock);
        let app = app_test(test_state).await.expect("Router creation failed");
        let server = TestServer::new(app).expect("Test server creation failed");

        let response = server.get("/status").await;

        response.assert_status_ok();
        response.assert_text("Alive !");
    }
}
