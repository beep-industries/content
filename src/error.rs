#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("HttpServerError: {0}")]
    HttpServer(String),
}
