#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("HttpServerError: {0}")]
    HttpServer(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("OpenTelemetryError: {0}")]
    OpenTelemetry(String),
}
