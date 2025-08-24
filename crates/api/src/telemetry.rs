use anyhow::Result;
use std::env;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Service information for telemetry
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub environment: String,
    pub instance_id: String,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        Self {
            name: "radarr-mvp",
            version: env!("CARGO_PKG_VERSION"),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            instance_id: format!("radarr-{}", uuid::Uuid::new_v4()),
        }
    }
}

/// OpenTelemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub service: ServiceInfo,
    pub otlp_endpoint: String,
    pub jaeger_endpoint: Option<String>,
    pub prometheus_endpoint: String,
    pub enable_logging: bool,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub log_level: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service: ServiceInfo::default(),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://otel-collector:4317".to_string()),
            jaeger_endpoint: env::var("JAEGER_ENDPOINT").ok(),
            prometheus_endpoint: env::var("PROMETHEUS_ENDPOINT")
                .unwrap_or_else(|_| "0.0.0.0:9090".to_string()),
            enable_logging: env::var("OTEL_LOGS_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            enable_metrics: env::var("OTEL_METRICS_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            enable_tracing: env::var("OTEL_TRACES_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }
}

/// Initialize telemetry with simplified configuration for MVP
pub fn init_telemetry(config: TelemetryConfig) -> Result<()> {
    // For MVP, use simple JSON logging with structured fields
    let filter = EnvFilter::from_env("RUST_LOG");

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .json(),
        )
        .with(filter)
        .init();

    tracing::info!(
        service.name = config.service.name,
        service.version = config.service.version,
        service.environment = config.service.environment,
        service.instance_id = config.service.instance_id,
        "Telemetry initialized successfully"
    );

    Ok(())
}

// Simplified for MVP - full OpenTelemetry integration can be added later

/// Shutdown telemetry gracefully
pub fn shutdown_telemetry() {
    tracing::info!("Telemetry shutdown complete");
}

// Simplified header extraction for MVP
pub fn extract_correlation_id(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-correlation-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

/// Generate a new trace ID for requests without one
pub fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Custom span creation for business operations
#[macro_export]
macro_rules! business_span {
    ($operation:expr, $($field:tt)*) => {
        tracing::info_span!(
            "business_operation",
            operation = $operation,
            otel.kind = "internal",
            $($field)*
        )
    };
}

/// Custom span for external service calls
#[macro_export]
macro_rules! external_span {
    ($service:expr, $operation:expr, $($field:tt)*) => {
        tracing::info_span!(
            "external_call",
            service = $service,
            operation = $operation,
            otel.kind = "client",
            $($field)*
        )
    };
}

/// Custom span for database operations
#[macro_export]
macro_rules! db_span {
    ($operation:expr, $table:expr, $($field:tt)*) => {
        tracing::info_span!(
            "db_operation",
            db.operation = $operation,
            db.table = $table,
            otel.kind = "client",
            $($field)*
        )
    };
}
