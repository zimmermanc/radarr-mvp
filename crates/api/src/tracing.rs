use anyhow::Result;
use axum::{
    body::Body,
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use tracing::{field, Instrument, Span};
use uuid::Uuid;

/// Simplified tracing utilities for Radarr MVP
pub struct DistributedTracing;

impl DistributedTracing {
    /// Extract correlation ID from headers
    pub fn extract_correlation_id(headers: &HeaderMap) -> Option<String> {
        headers
            .get("x-correlation-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
    
    /// Generate a new correlation ID
    pub fn generate_correlation_id() -> String {
        Uuid::new_v4().to_string()
    }
    
    /// Create a span for business operations
    pub fn business_span(operation: &str, entity_type: &str, entity_id: Option<&str>) -> Span {
        tracing::info_span!(
            "business_operation",
            operation = operation,
            entity_type = entity_type,
            entity_id = entity_id.unwrap_or(""),
            duration_ms = field::Empty,
            success = field::Empty,
        )
    }
    
    /// Create a span for database operations
    pub fn database_span(operation: &str, table: &str) -> Span {
        tracing::info_span!(
            "database_operation",
            operation = operation,
            table = table,
            duration_ms = field::Empty,
            success = field::Empty,
        )
    }
    
    /// Create a span for external service calls
    pub fn external_span(service: &str, operation: &str) -> Span {
        tracing::info_span!(
            "external_call",
            service = service,
            operation = operation,
            duration_ms = field::Empty,
            success = field::Empty,
        )
    }
}

/// Simple tracing middleware for MVP
pub async fn simple_tracing_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let headers = req.headers();
    
    // Get or generate correlation ID
    let correlation_id = DistributedTracing::extract_correlation_id(headers)
        .unwrap_or_else(|| DistributedTracing::generate_correlation_id());
    
    req.extensions_mut().insert(correlation_id.clone());
    
    let start = std::time::Instant::now();
    
    // Create span for the request
    let span = tracing::info_span!(
        "http_request",
        correlation_id = %correlation_id,
        method = %req.method(),
        path = %req.uri().path(),
        status_code = field::Empty,
        duration_ms = field::Empty,
    );
    
    let _enter = span.enter();
    let result = next.run(req).await;
    let duration = start.elapsed();
    
    span.record("status_code", result.status().as_u16());
    span.record("duration_ms", duration.as_millis() as u64);
    
    Ok(result)
}

/// Business operation instrumentation helper
pub async fn instrument_business_operation<T, F>(
    operation: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    operation_fn: F,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let span = DistributedTracing::business_span(operation, entity_type, entity_id);
    let _enter = span.enter();
    
    let start = std::time::Instant::now();
    let result = operation_fn.await;
    let duration = start.elapsed();
    
    span.record("duration_ms", duration.as_millis() as u64);
    
    match &result {
        Ok(_) => {
            span.record("success", true);
            tracing::info!("Business operation completed successfully");
        }
        Err(e) => {
            span.record("success", false);
            tracing::error!("Business operation failed: {}", e);
        }
    }
    
    result
}

/// Correlation ID utilities
pub struct CorrelationId;

impl CorrelationId {
    /// Generate a new correlation ID
    pub fn new() -> String {
        Uuid::new_v4().to_string()
    }
    
    /// Extract correlation ID from headers
    pub fn from_headers(headers: &HeaderMap) -> Option<String> {
        headers
            .get("x-correlation-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
}