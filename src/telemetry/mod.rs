//! Telemetry and observability module
//!
//! Provides comprehensive observability including:
//! - Structured logging with tracing
//! - Prometheus metrics
//! - OpenTelemetry integration
//! - Request tracing
//! - Log-trace correlation

mod metrics;
mod tracing_ext;

pub use metrics::*;
pub use tracing_ext::*;

use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::TelemetryConfig;
use crate::error::SimulatorResult;

/// Initialize the telemetry subsystem
pub fn init_telemetry(config: &TelemetryConfig) -> SimulatorResult<()> {
    if !config.enabled {
        return Ok(());
    }

    // Build the env filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Build subscriber based on config
    let subscriber = tracing_subscriber::registry()
        .with(env_filter);

    if config.json_logs {
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(true);

        subscriber.with(json_layer).init();
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_file(false)
            .with_line_number(false)
            .with_target(true)
            .compact();

        subscriber.with(fmt_layer).init();
    }

    tracing::info!(
        service = %config.service_name,
        version = %env!("CARGO_PKG_VERSION"),
        "Telemetry initialized"
    );

    Ok(())
}

/// Initialize OpenTelemetry tracing (optional)
#[cfg(feature = "otel")]
pub fn init_otel(config: &TelemetryConfig) -> SimulatorResult<()> {
    use opentelemetry::sdk::trace::{self, RandomIdGenerator, Sampler};
    use opentelemetry_otlp::WithExportConfig;

    if let Some(endpoint) = &config.otlp_endpoint {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint)
            )
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(opentelemetry::sdk::Resource::new(vec![
                        opentelemetry::KeyValue::new("service.name", config.service_name.clone()),
                        opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ]))
            )
            .install_batch(opentelemetry::runtime::Tokio)
            .map_err(|e| crate::error::SimulationError::Config(e.to_string()))?;

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        // Note: This needs to be integrated with the main subscriber
        tracing::info!("OpenTelemetry tracing initialized with endpoint: {}", endpoint);
    }

    Ok(())
}

/// Shutdown telemetry gracefully
pub fn shutdown_telemetry() {
    #[cfg(feature = "otel")]
    opentelemetry::global::shutdown_tracer_provider();

    tracing::info!("Telemetry shutdown complete");
}

/// Log a request start
#[inline]
pub fn log_request_start(request_id: &str, method: &str, path: &str) {
    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        "Request started"
    );
}

/// Log a request completion
#[inline]
pub fn log_request_end(
    request_id: &str,
    status: u16,
    latency: Duration,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
) {
    tracing::info!(
        request_id = %request_id,
        status = status,
        latency_ms = latency.as_millis() as u64,
        input_tokens = input_tokens,
        output_tokens = output_tokens,
        "Request completed"
    );
}

/// Log an error
#[inline]
pub fn log_error(request_id: &str, error: &str, error_type: &str) {
    tracing::error!(
        request_id = %request_id,
        error = %error,
        error_type = %error_type,
        "Request failed"
    );
}

/// Create a span for tracing
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        tracing::info_span!($name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!($name, $($field)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_telemetry() {
        // Can only init once, so this test is limited
        let config = TelemetryConfig {
            enabled: false,
            ..Default::default()
        };

        assert!(init_telemetry(&config).is_ok());
    }
}
