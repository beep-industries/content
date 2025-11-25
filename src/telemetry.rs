use dotenv::dotenv;
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_sdk::{
    metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider},
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::{
    attribute::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing_core::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::Config;
use crate::error::TelemetryError;

fn resource() -> Resource {
    Resource::builder()
        .with_service_name(env!("CARGO_PKG_NAME"))
        .with_schema_url(
            [
                KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "develop"),
            ],
            SCHEMA_URL,
        )
        .build()
}

fn init_meter_provider() -> Result<SdkMeterProvider, TelemetryError> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()
        .map_err(|e| TelemetryError::OpenTelemetry(format!("failed to build OTLP metric exporter: {}", e)))?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let stdout_reader =
        PeriodicReader::builder(opentelemetry_stdout::MetricExporter::default()).build();

    let meter_provider = MeterProviderBuilder::default()
        .with_resource(resource())
        .with_reader(reader)
        .with_reader(stdout_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}

fn init_tracer_provider() -> Result<SdkTracerProvider, TelemetryError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .map_err(|e| TelemetryError::OpenTelemetry(format!("failed to build OTLP span exporter: {}", e)))?;
    Ok(SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build())
}

fn init_tracing_subscriber() -> Result<OtelGuard, TelemetryError> {
    let tracer_provider = init_tracer_provider()?;
    let meter_provider = init_meter_provider()?;

    let tracer = tracer_provider.tracer("tracing-otel-subscriber");

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    Ok(OtelGuard {
        tracer_provider,
        meter_provider,
    })
}

struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl OtelGuard {
    /// Shutdown providers and flush any buffered telemetry.
    pub async fn shutdown(self) {
        let tracer_provider = self.tracer_provider;
        let meter_provider = self.meter_provider;

        let _ = tokio::task::spawn_blocking(move || {
            if let Err(err) = tracer_provider.shutdown() {
                eprintln!("tracer shutdown error: {err:?}");
            }
            if let Err(err) = meter_provider.shutdown() {
                eprintln!("meter shutdown error: {err:?}");
            }
        })
        .await;
    }
}

pub async fn run(config: Config) -> Result<(), TelemetryError> {
    dotenv().ok();

    let guard = init_tracing_subscriber()?;

    let _ = crate::http::serve(config)
        .await
        .inspect_err(|e| eprintln!("{}", e));

    guard.shutdown().await;

    Ok(())
}
