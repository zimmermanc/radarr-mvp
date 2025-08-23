//! Correlation ID management for distributed tracing
//!
//! This module provides correlation IDs to track requests and operations
//! across multiple components and services, enabling better debugging and monitoring.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A correlation ID used to track operations across the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(Uuid);

impl CorrelationId {
    /// Create a new random correlation ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a correlation ID from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a correlation ID from a string
    pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Convert to a hyphenated string
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    /// Convert to a simple string (no hyphens)
    pub fn to_simple(&self) -> String {
        self.0.simple().to_string()
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CorrelationId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<CorrelationId> for Uuid {
    fn from(id: CorrelationId) -> Self {
        id.0
    }
}

/// Context that carries correlation information through operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationContext {
    /// The main correlation ID for this operation
    pub correlation_id: CorrelationId,
    
    /// Parent correlation ID if this is a sub-operation
    pub parent_id: Option<CorrelationId>,
    
    /// Originating service/component name
    pub origin: String,
    
    /// Optional user ID associated with this operation
    pub user_id: Option<String>,
    
    /// Optional session ID
    pub session_id: Option<String>,
    
    /// Operation start timestamp
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl CorrelationContext {
    /// Create a new correlation context
    pub fn new(origin: impl Into<String>) -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            parent_id: None,
            origin: origin.into(),
            user_id: None,
            session_id: None,
            started_at: chrono::Utc::now(),
        }
    }

    /// Create a child context from this one
    pub fn child(&self, origin: impl Into<String>) -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            parent_id: Some(self.correlation_id),
            origin: origin.into(),
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            started_at: chrono::Utc::now(),
        }
    }

    /// Add user context
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Add session context
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Get the elapsed time since this context was created
    pub fn elapsed(&self) -> chrono::Duration {
        chrono::Utc::now() - self.started_at
    }

    /// Format for logging
    pub fn to_log_string(&self) -> String {
        format!(
            "correlation_id={} parent_id={} origin={} elapsed_ms={}",
            self.correlation_id,
            self.parent_id.map(|id| id.to_string()).unwrap_or_else(|| "none".to_string()),
            self.origin,
            self.elapsed().num_milliseconds()
        )
    }
}

/// Thread-local storage for the current correlation context
thread_local! {
    static CURRENT_CONTEXT: std::cell::RefCell<Option<CorrelationContext>> = std::cell::RefCell::new(None);
}

/// Set the current correlation context for this thread
pub fn set_current_context(context: CorrelationContext) {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(context);
    });
}

/// Get the current correlation context for this thread
pub fn current_context() -> Option<CorrelationContext> {
    CURRENT_CONTEXT.with(|c| c.borrow().clone())
}

/// Get the current correlation ID, or create a new one if none exists
pub fn current_correlation_id() -> CorrelationId {
    CURRENT_CONTEXT.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.correlation_id)
            .unwrap_or_else(CorrelationId::new)
    })
}

/// Clear the current correlation context
pub fn clear_context() {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = None;
    });
}

/// Execute a function with a specific correlation context
pub async fn with_context<F, Fut, T>(context: CorrelationContext, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    set_current_context(context);
    let result = f().await;
    clear_context();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_id_creation() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1, id2);
        
        let uuid = Uuid::new_v4();
        let id3 = CorrelationId::from_uuid(uuid);
        assert_eq!(id3.as_uuid(), uuid);
    }

    #[test]
    fn test_correlation_id_parsing() {
        let id = CorrelationId::new();
        let s = id.to_string();
        let parsed = CorrelationId::parse_str(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_correlation_context() {
        let ctx = CorrelationContext::new("test-service");
        assert_eq!(ctx.origin, "test-service");
        assert!(ctx.parent_id.is_none());
        
        let child = ctx.child("child-service");
        assert_eq!(child.origin, "child-service");
        assert_eq!(child.parent_id, Some(ctx.correlation_id));
    }

    #[test]
    fn test_context_with_user_and_session() {
        let ctx = CorrelationContext::new("api")
            .with_user("user123")
            .with_session("session456");
        
        assert_eq!(ctx.user_id, Some("user123".to_string()));
        assert_eq!(ctx.session_id, Some("session456".to_string()));
    }

    #[tokio::test]
    async fn test_thread_local_context() {
        let ctx = CorrelationContext::new("test");
        let id = ctx.correlation_id;
        
        set_current_context(ctx);
        assert_eq!(current_correlation_id(), id);
        
        let retrieved = current_context().unwrap();
        assert_eq!(retrieved.correlation_id, id);
        
        clear_context();
        assert!(current_context().is_none());
    }

    #[tokio::test]
    async fn test_with_context() {
        let ctx = CorrelationContext::new("test");
        let id = ctx.correlation_id;
        
        let result = with_context(ctx, || async {
            current_correlation_id()
        }).await;
        
        assert_eq!(result, id);
        assert!(current_context().is_none()); // Should be cleared after
    }
}