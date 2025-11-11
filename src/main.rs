use clap::Parser;
use dotenv::dotenv;

use crate::config::Config;

use opentelemetry::trace::TracerProvider as _;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing::instrument;


mod config;
mod error;
mod http;

#[instrument(level = "info", skip(config))]
async fn startup_check(config: &Config) {
    tracing::info!("Tracing system initialized successfully!");
    crate::http::serve(config.clone())
        .await
        .inspect_err(|e| tracing::error!("{}", e))
        .ok();
}


#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = Config::parse();

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build();
    let tracer = provider.tracer("content_service_tracer");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = tracing_subscriber::fmt::layer().with_span_events(
        FmtSpan::ENTER | FmtSpan::CLOSE,
    );

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(fmt_layer)
        .with(telemetry)
        .init();

    startup_check(&config).await;
}
