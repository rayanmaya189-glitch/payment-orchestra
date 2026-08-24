//! Platform telemetry initialization — structured JSON logging + OpenTelemetry tracing.
//!
//! ## Architecture
//!
//! All services call `platform_logging::telemetry::init()` at startup. This sets up:
//!
//! 1. **Structured JSON logging** via `tracing-subscriber` with `EnvFilter`
//! 2. **OpenTelemetry distributed tracing** via `opentelemetry-otlp` (if configured)
//!
//! ## OpenTelemetry Configuration
//!
//! OpenTelemetry is enabled only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set.
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` — OTLP collector endpoint (e.g., http://localhost:4317)
//! - `OTEL_SERVICE_NAME` — Service name (defaults to crate name)
//! - `OTEL_BATCH_INTERVAL_MS` — Batch span export interval (default: 5000ms)

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

/// Initialize the platform telemetry stack.
///
/// Sets up:
/// - Structured JSON logging to stdout
/// - Environment-variable controlled log filtering (via `RUST_LOG`)
/// - OpenTelemetry distributed tracing via OTLP (if `OTEL_EXPORTER_OTLP_ENDPOINT` is set)
pub fn init() {
    // Build the JSON logging layer
    let json_layer = fmt::layer()
        .json()
        .with_target(false)
        .with_current_span(true)
        .with_span_list(true);

    // Build the env filter layer (from RUST_LOG env var, defaults to "info")
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Start with the registry and always-on layers
    let subscriber = Registry::default()
        .with(json_layer)
        .with(env_filter);

    // Try to initialize OpenTelemetry tracing (optional, non-fatal)
    if let Some((tracer, provider)) = init_opentelemetry_tracer() {
        // Register the tracer provider globally so shutdown() can flush spans
        opentelemetry::global::set_tracer_provider(provider);

        subscriber
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();
        tracing::info!("Telemetry initialized with OpenTelemetry OTLP tracing");
    } else {
        subscriber.init();
        tracing::info!("Telemetry initialized (structured JSON logging only)");
    }
}

/// Initialize an OpenTelemetry tracer if the OTLP endpoint is configured.
///
/// Returns `Some((opentelemetry_sdk::trace::Tracer, opentelemetry_sdk::trace::TracerProvider))`
/// if successful, `None` otherwise. This is intentionally non-fatal — if the OTLP collector
/// is unavailable, the service continues with structured logging only.
fn init_opentelemetry_tracer(
) -> Option<(
    opentelemetry_sdk::trace::Tracer,
    opentelemetry_sdk::trace::TracerProvider,
)> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty())?;

    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| env!("CARGO_PKG_NAME").to_string());

    tracing::info!(
        otlp_endpoint = %otlp_endpoint,
        service_name = %service_name,
        "Initializing OpenTelemetry tracing"
    );

    // Build the OTLP span exporter
    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&otlp_endpoint)
        .with_timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to build OTLP span exporter, continuing without distributed tracing"
            );
            return None;
        }
    };

    // Build the batch span processor (background exporting)
    let batch_processor = opentelemetry_sdk::trace::BatchSpanProcessor::builder(
        exporter,
        opentelemetry_sdk::runtime::Tokio,
    )
    .build();

    // Build the resource describing this service
    let resource = opentelemetry_sdk::Resource::new([
        KeyValue::new("service.name", service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION").to_string()),
    ]);

    // Build the tracer provider with the batch processor
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_resource(resource)
        .build();

    // Get a tracer from the provider
    let tracer = provider.tracer(service_name);

    tracing::info!("OpenTelemetry OTLP tracer installed successfully");
    Some((tracer, provider))
}

/// Force-flush and shut down the OpenTelemetry tracer provider.
///
/// Call during service graceful shutdown to ensure all spans are exported.
pub fn shutdown() {
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("OpenTelemetry tracer provider shut down");
}
