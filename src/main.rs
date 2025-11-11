use clap::Parser;
use dotenv::dotenv;
use std::env;
use tokio::time::Duration;
use tracing::instrument;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler};
use opentelemetry_sdk::{Resource};
use tonic::metadata::MetadataMap;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry::trace::TracerProvider;

use crate::config::Config;

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
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let config = Config::parse();

    // Endpoint OTLP
    let otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "otel-collector:4317".to_string());

    // Métadonnées GRPC (optionnelles)
    let map = MetadataMap::with_capacity(3);

    // --- CONFIGURATION OTEL ---
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .with_timeout(Duration::from_secs(3))
        .with_metadata(map)
        .build()?;

    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(Resource::builder()
            .with_attributes(vec![KeyValue::new("service.name", "content")])
            .build())
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer("beep-content-tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(fmt_layer)
        .with(telemetry)
        .init();

    startup_check(&config).await;
    
    tracing::info!("Shutting down OpenTelemetry tracer provider...");
    if let Err(err) = tracer_provider.shutdown() {
        tracing::error!(%err, "Failed to shut down tracer provider");
    } else {
        tracing::info!("Tracer provider shut down successfully");
    }

    Ok(())
}
