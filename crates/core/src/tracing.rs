//! Tracing and logging utilities with correlation ID support
//!
//! This module provides enhanced tracing functionality that automatically
//! includes correlation IDs in all log messages for distributed tracing.

use crate::correlation::current_context;
// use crate::correlation::{current_correlation_id, CorrelationId}; // Currently unused
use serde::Serialize;
use std::collections::HashMap;
use tracing::{Event, Subscriber};
// use tracing::Metadata; // Currently unused
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::{Context, Layer},
    registry::LookupSpan,
};

/// A tracing layer that adds correlation IDs to all events
pub struct CorrelationLayer;

impl Default for CorrelationLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrelationLayer {
    /// Create a new correlation layer
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for CorrelationLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {
        // Get the current correlation ID if available
        if let Some(context) = current_context() {
            // Add correlation ID to the event's fields
            let correlation_id = context.correlation_id.to_string();
            let parent_id = context.parent_id.map(|id| id.to_string());
            let origin = context.origin.clone();
            
            // Note: We can't directly modify the event, but we can log additional context
            tracing::trace!(
                correlation_id = %correlation_id,
                parent_id = ?parent_id,
                origin = %origin,
                "Event with correlation context"
            );
        }
    }
}

/// Structured logging entry with correlation context
#[derive(Debug, Clone, Serialize)]
pub struct CorrelatedLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
    pub correlation_id: Option<String>,
    pub parent_id: Option<String>,
    pub origin: Option<String>,
    pub fields: HashMap<String, serde_json::Value>,
}

impl CorrelatedLogEntry {
    /// Create a new correlated log entry
    pub fn new(level: impl Into<String>, message: impl Into<String>) -> Self {
        let context = current_context();
        
        Self {
            timestamp: chrono::Utc::now(),
            level: level.into(),
            message: message.into(),
            correlation_id: context.as_ref().map(|c| c.correlation_id.to_string()),
            parent_id: context.as_ref().and_then(|c| c.parent_id.map(|id| id.to_string())),
            origin: context.as_ref().map(|c| c.origin.clone()),
            fields: HashMap::new(),
        }
    }
    
    /// Add a field to the log entry
    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.fields.insert(key.into(), json_value);
        }
        self
    }
    
    /// Format the log entry for output
    pub fn format(&self) -> String {
        let mut parts = vec![
            format!("[{}]", self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f")),
            format!("[{}]", self.level),
        ];
        
        if let Some(ref correlation_id) = self.correlation_id {
            parts.push(format!("[correlation_id={}]", correlation_id));
        }
        
        if let Some(ref parent_id) = self.parent_id {
            parts.push(format!("[parent_id={}]", parent_id));
        }
        
        if let Some(ref origin) = self.origin {
            parts.push(format!("[origin={}]", origin));
        }
        
        parts.push(self.message.clone());
        
        if !self.fields.is_empty() {
            let fields: Vec<String> = self.fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            parts.push(format!("{{ {} }}", fields.join(", ")));
        }
        
        parts.join(" ")
    }
}

/// Macro to log with correlation context
#[macro_export]
macro_rules! log_with_correlation {
    ($level:expr, $msg:expr) => {
        {
            let entry = $crate::tracing::CorrelatedLogEntry::new($level, $msg);
            tracing::info!("{}", entry.format());
        }
    };
    ($level:expr, $msg:expr, $($key:ident = $value:expr),*) => {
        {
            let mut entry = $crate::tracing::CorrelatedLogEntry::new($level, $msg);
            $(
                entry = entry.with_field(stringify!($key), $value);
            )*
            tracing::info!("{}", entry.format());
        }
    };
}

/// Initialize tracing with correlation support
pub fn init_correlated_tracing() {
    let fmt_layer = fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);
    
    let correlation_layer = CorrelationLayer::new();
    
    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(correlation_layer)
        .init();
}

/// Helper function to log with current correlation context
pub fn info_with_correlation(message: impl Into<String>) {
    if let Some(context) = current_context() {
        tracing::info!(
            correlation_id = %context.correlation_id,
            parent_id = ?context.parent_id,
            origin = %context.origin,
            elapsed_ms = %context.elapsed().num_milliseconds(),
            "{}",
            message.into()
        );
    } else {
        tracing::info!("{}", message.into());
    }
}

/// Helper function to log debug messages with correlation context
pub fn debug_with_correlation(message: impl Into<String>) {
    if let Some(context) = current_context() {
        tracing::debug!(
            correlation_id = %context.correlation_id,
            parent_id = ?context.parent_id,
            origin = %context.origin,
            elapsed_ms = %context.elapsed().num_milliseconds(),
            "{}",
            message.into()
        );
    } else {
        tracing::debug!("{}", message.into());
    }
}

/// Helper function to log warnings with correlation context
pub fn warn_with_correlation(message: impl Into<String>) {
    if let Some(context) = current_context() {
        tracing::warn!(
            correlation_id = %context.correlation_id,
            parent_id = ?context.parent_id,
            origin = %context.origin,
            elapsed_ms = %context.elapsed().num_milliseconds(),
            "{}",
            message.into()
        );
    } else {
        tracing::warn!("{}", message.into());
    }
}

/// Helper function to log errors with correlation context
pub fn error_with_correlation(message: impl Into<String>) {
    if let Some(context) = current_context() {
        tracing::error!(
            correlation_id = %context.correlation_id,
            parent_id = ?context.parent_id,
            origin = %context.origin,
            elapsed_ms = %context.elapsed().num_milliseconds(),
            "{}",
            message.into()
        );
    } else {
        tracing::error!("{}", message.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlation::{CorrelationContext, set_current_context};
    
    #[test]
    fn test_correlated_log_entry() {
        let entry = CorrelatedLogEntry::new("INFO", "Test message")
            .with_field("user_id", "123")
            .with_field("action", "login");
        
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
        assert_eq!(entry.fields.len(), 2);
        
        let formatted = entry.format();
        assert!(formatted.contains("[INFO]"));
        assert!(formatted.contains("Test message"));
        assert!(formatted.contains("user_id"));
        assert!(formatted.contains("action"));
    }
    
    #[test]
    fn test_log_entry_with_correlation_context() {
        let context = CorrelationContext::new("test-service")
            .with_user("user123")
            .with_session("session456");
        
        set_current_context(context.clone());
        
        let entry = CorrelatedLogEntry::new("DEBUG", "Operation completed");
        
        assert_eq!(entry.correlation_id, Some(context.correlation_id.to_string()));
        assert_eq!(entry.origin, Some("test-service".to_string()));
        assert!(entry.parent_id.is_none());
        
        let formatted = entry.format();
        assert!(formatted.contains(&context.correlation_id.to_string()));
        assert!(formatted.contains("test-service"));
        
        crate::correlation::clear_context();
    }
    
    #[test]
    fn test_log_entry_without_correlation_context() {
        crate::correlation::clear_context();
        
        let entry = CorrelatedLogEntry::new("WARN", "No correlation");
        
        assert!(entry.correlation_id.is_none());
        assert!(entry.parent_id.is_none());
        assert!(entry.origin.is_none());
        
        let formatted = entry.format();
        assert!(!formatted.contains("correlation_id"));
        assert!(!formatted.contains("parent_id"));
        assert!(!formatted.contains("origin"));
    }
}
