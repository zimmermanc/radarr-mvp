//! Monitoring infrastructure for List Sync system
//!
//! This module provides comprehensive monitoring, metrics, and alerting
//! for the List Sync system, including:
//!
//! - Performance metrics tracking
//! - Health checks for external services
//! - Circuit breaker integration
//! - Alert management for failures and rate limits
//! - Prometheus metrics export

pub mod alert_manager;
pub mod health_checks;
pub mod list_sync_monitor;
pub mod metrics;

pub use alert_manager::{Alert, AlertLevel, AlertManager, AlertRule};
pub use health_checks::{HealthChecker, HealthStatus, ServiceHealth};
pub use list_sync_monitor::ListSyncMonitor;
pub use metrics::{PrometheusMetrics, ServiceMetrics, SyncMetrics};
