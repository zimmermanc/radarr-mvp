//! Alert management system for List Sync monitoring
//!
//! This module provides alerting capabilities for the List Sync system,
//! including rule-based alerting, escalation, and notification integration.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertLevel {
    /// Informational alerts for non-critical events
    Info,
    /// Warning alerts for potential issues
    Warning,
    /// Critical alerts requiring immediate attention
    Critical,
    /// Emergency alerts for system-wide failures
    Emergency,
}

impl AlertLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Emergency => "emergency",
        }
    }

    /// Get numeric priority for sorting (higher = more urgent)
    pub fn priority(&self) -> u8 {
        match self {
            Self::Info => 1,
            Self::Warning => 2,
            Self::Critical => 3,
            Self::Emergency => 4,
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Alert is active and firing
    Active,
    /// Alert condition resolved
    Resolved,
    /// Alert was manually acknowledged
    Acknowledged,
    /// Alert was suppressed (e.g., during maintenance)
    Suppressed,
}

/// Individual alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub rule_name: String,
    pub level: AlertLevel,
    pub status: AlertStatus,
    pub title: String,
    pub description: String,
    pub service: String,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub fire_count: u32,
    pub last_fired: DateTime<Utc>,
}

impl Alert {
    pub fn new(
        rule: &AlertRule,
        service: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            rule_name: rule.name.clone(),
            level: rule.level,
            status: AlertStatus::Active,
            title: title.into(),
            description: description.into(),
            service: service.into(),
            labels: rule.labels.clone(),
            created_at: now,
            updated_at: now,
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            fire_count: 1,
            last_fired: now,
        }
    }

    /// Check if alert should be suppressed due to rate limiting
    pub fn should_rate_limit(&self, rule: &AlertRule) -> bool {
        if let Some(rate_limit) = rule.rate_limit {
            let time_since_last_fire = Utc::now() - self.last_fired;
            time_since_last_fire < rate_limit
        } else {
            false
        }
    }

    /// Update alert status and timestamps
    pub fn update_status(&mut self, status: AlertStatus, user: Option<String>) {
        self.status = status;
        self.updated_at = Utc::now();

        match status {
            AlertStatus::Resolved => {
                self.resolved_at = Some(Utc::now());
            }
            AlertStatus::Acknowledged => {
                self.acknowledged_at = Some(Utc::now());
                self.acknowledged_by = user;
            }
            _ => {}
        }
    }

    /// Fire the alert again (increment count and update timestamp)
    pub fn fire(&mut self) {
        self.fire_count += 1;
        self.last_fired = Utc::now();
        self.updated_at = Utc::now();
        self.status = AlertStatus::Active;
    }
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub level: AlertLevel,
    pub description: String,
    pub labels: HashMap<String, String>,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub evaluation_window: Duration,
    pub rate_limit: Option<Duration>,
    pub auto_resolve: bool,
    pub auto_resolve_after: Option<Duration>,
    pub enabled: bool,
}

/// Alert rule conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Consecutive sync failures
    ConsecutiveFailures { service: String, count: u32 },
    /// Sync duration exceeds threshold
    SlowSync {
        service: String,
        duration_seconds: f64,
    },
    /// Rate limit hit frequency
    RateLimitHits { service: String, hits_per_hour: u32 },
    /// Service unavailable
    ServiceDown {
        service: String,
        check_interval_seconds: u64,
    },
    /// Cache hit rate below threshold
    LowCacheHitRate { cache_type: String, threshold: f64 },
    /// Queue depth exceeds threshold
    HighQueueDepth {
        queue_name: String,
        depth_threshold: u64,
    },
    /// Circuit breaker open
    CircuitBreakerOpen { service: String },
}

/// Alert manager for handling rule evaluation and notifications
pub struct AlertManager {
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<Uuid, Alert>>>,
    alert_history: Arc<RwLock<Vec<Alert>>>,
    notification_handlers: Vec<Box<dyn AlertNotificationHandler>>,
}

/// Trait for handling alert notifications
#[async_trait::async_trait]
pub trait AlertNotificationHandler: Send + Sync {
    async fn send_notification(&self, alert: &Alert) -> Result<(), AlertError>;
    fn name(&self) -> &str;
}

/// Alert system errors
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    #[error("Alert not found: {0}")]
    AlertNotFound(Uuid),

    #[error("Notification error: {0}")]
    NotificationError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

impl AlertManager {
    /// Create a new AlertManager
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            notification_handlers: Vec::new(),
        }
    }

    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) -> Result<(), AlertError> {
        let mut rules = self.rules.write().await;

        if !rule.enabled {
            debug!("Adding disabled rule: {}", rule.name);
        }

        info!(
            "Adding alert rule: {} (level: {})",
            rule.name,
            rule.level.as_str()
        );
        rules.insert(rule.name.clone(), rule);
        Ok(())
    }

    /// Remove an alert rule
    pub async fn remove_rule(&self, rule_name: &str) -> Result<(), AlertError> {
        let mut rules = self.rules.write().await;
        if rules.remove(rule_name).is_some() {
            info!("Removed alert rule: {}", rule_name);
            Ok(())
        } else {
            Err(AlertError::RuleNotFound(rule_name.to_string()))
        }
    }

    /// Enable or disable a rule
    pub async fn set_rule_enabled(&self, rule_name: &str, enabled: bool) -> Result<(), AlertError> {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.get_mut(rule_name) {
            rule.enabled = enabled;
            info!("Set rule '{}' enabled={}", rule_name, enabled);
            Ok(())
        } else {
            Err(AlertError::RuleNotFound(rule_name.to_string()))
        }
    }

    /// Check for consecutive sync failures
    pub async fn check_consecutive_failures(&self, service: &str, failure_count: u32) {
        let rules = self.rules.read().await;

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let AlertCondition::ConsecutiveFailures {
                service: rule_service,
                count,
            } = &rule.condition
            {
                if rule_service == service && failure_count >= *count {
                    let title = format!("{} sync failures", failure_count);
                    let description = format!(
                        "Service {} has failed {} consecutive sync attempts",
                        service, failure_count
                    );

                    self.fire_alert(rule, service, title, description).await;
                }
            }
        }
    }

    /// Check for slow sync operations
    pub async fn check_slow_sync(&self, service: &str, duration_seconds: f64) {
        let rules = self.rules.read().await;

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let AlertCondition::SlowSync {
                service: rule_service,
                duration_seconds: threshold,
            } = &rule.condition
            {
                if rule_service == service && duration_seconds > *threshold {
                    let title = "Slow sync operation".to_string();
                    let description = format!(
                        "Service {} sync took {:.2}s (threshold: {:.2}s)",
                        service, duration_seconds, threshold
                    );

                    self.fire_alert(rule, service, title, description).await;
                }
            }
        }
    }

    /// Check for rate limit hits
    pub async fn check_rate_limit_hits(&self, service: &str, hits_per_hour: u32) {
        let rules = self.rules.read().await;

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let AlertCondition::RateLimitHits {
                service: rule_service,
                hits_per_hour: threshold,
            } = &rule.condition
            {
                if rule_service == service && hits_per_hour >= *threshold {
                    let title = "High rate limit hits".to_string();
                    let description = format!(
                        "Service {} hit rate limits {} times per hour (threshold: {})",
                        service, hits_per_hour, threshold
                    );

                    self.fire_alert(rule, service, title, description).await;
                }
            }
        }
    }

    /// Check service health
    pub async fn check_service_health(&self, service: &str, is_healthy: bool) {
        let rules = self.rules.read().await;

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let AlertCondition::ServiceDown {
                service: rule_service,
                ..
            } = &rule.condition
            {
                if rule_service == service && !is_healthy {
                    let title = "Service unavailable".to_string();
                    let description =
                        format!("Service {} is not responding to health checks", service);

                    self.fire_alert(rule, service, title, description).await;
                } else if rule_service == service && is_healthy {
                    // Auto-resolve service down alerts when service comes back up
                    self.resolve_alerts_for_condition(rule, service).await;
                }
            }
        }
    }

    /// Check circuit breaker state
    pub async fn check_circuit_breaker_state(&self, service: &str, is_open: bool) {
        let rules = self.rules.read().await;

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if let AlertCondition::CircuitBreakerOpen {
                service: rule_service,
            } = &rule.condition
            {
                if rule_service == service && is_open {
                    let title = "Circuit breaker open".to_string();
                    let description = format!("Circuit breaker for service {} is open", service);

                    self.fire_alert(rule, service, title, description).await;
                } else if rule_service == service && !is_open {
                    // Auto-resolve circuit breaker alerts when circuit closes
                    self.resolve_alerts_for_condition(rule, service).await;
                }
            }
        }
    }

    /// Fire an alert based on a rule
    async fn fire_alert(
        &self,
        rule: &AlertRule,
        service: &str,
        title: String,
        description: String,
    ) {
        let mut active_alerts = self.active_alerts.write().await;

        // Check if we already have an active alert for this rule and service
        let existing_alert = active_alerts.values_mut().find(|alert| {
            alert.rule_name == rule.name
                && alert.service == service
                && alert.status == AlertStatus::Active
        });

        match existing_alert {
            Some(existing) => {
                // Check rate limiting
                if existing.should_rate_limit(rule) {
                    debug!(
                        rule = rule.name,
                        service = service,
                        "Alert rate limited, not firing again"
                    );
                    return;
                }

                // Re-fire existing alert
                existing.fire();
                warn!(
                    rule = rule.name,
                    service = service,
                    fire_count = existing.fire_count,
                    "Alert fired again"
                );
            }
            None => {
                // Create new alert
                let alert = Alert::new(rule, service, title, description);
                warn!(
                    alert_id = %alert.id,
                    rule = rule.name,
                    service = service,
                    level = rule.level.as_str(),
                    "New alert fired"
                );

                active_alerts.insert(alert.id, alert);
            }
        }
    }

    /// Resolve alerts for a specific condition and service
    async fn resolve_alerts_for_condition(&self, rule: &AlertRule, service: &str) {
        let mut active_alerts = self.active_alerts.write().await;
        let mut resolved_count = 0;

        for alert in active_alerts.values_mut() {
            if alert.rule_name == rule.name
                && alert.service == service
                && alert.status == AlertStatus::Active
            {
                alert.update_status(AlertStatus::Resolved, None);
                resolved_count += 1;

                info!(
                    alert_id = %alert.id,
                    rule = rule.name,
                    service = service,
                    "Alert automatically resolved"
                );
            }
        }

        if resolved_count > 0 {
            debug!(
                rule = rule.name,
                service = service,
                count = resolved_count,
                "Auto-resolved alerts"
            );
        }
    }

    /// Manually acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: Uuid, user: String) -> Result<(), AlertError> {
        let mut active_alerts = self.active_alerts.write().await;

        if let Some(alert) = active_alerts.get_mut(&alert_id) {
            alert.update_status(AlertStatus::Acknowledged, Some(user.clone()));
            info!(
                alert_id = %alert_id,
                user = user,
                "Alert acknowledged"
            );
            Ok(())
        } else {
            Err(AlertError::AlertNotFound(alert_id))
        }
    }

    /// Manually resolve an alert
    pub async fn resolve_alert(&self, alert_id: Uuid, user: String) -> Result<(), AlertError> {
        let mut active_alerts = self.active_alerts.write().await;

        if let Some(alert) = active_alerts.get_mut(&alert_id) {
            alert.update_status(AlertStatus::Resolved, Some(user.clone()));
            info!(
                alert_id = %alert_id,
                user = user,
                "Alert manually resolved"
            );
            Ok(())
        } else {
            Err(AlertError::AlertNotFound(alert_id))
        }
    }

    /// Get all active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let active_alerts = self.active_alerts.read().await;
        let mut alerts: Vec<Alert> = active_alerts
            .values()
            .filter(|alert| alert.status == AlertStatus::Active)
            .cloned()
            .collect();

        // Sort by priority (highest first), then by creation time (newest first)
        alerts.sort_by(|a, b| {
            b.level
                .priority()
                .cmp(&a.level.priority())
                .then_with(|| b.created_at.cmp(&a.created_at))
        });

        alerts
    }

    /// Get alert statistics
    pub async fn get_alert_stats(&self) -> AlertStats {
        let active_alerts = self.active_alerts.read().await;
        let history = self.alert_history.read().await;

        let active_by_level = active_alerts
            .values()
            .filter(|alert| alert.status == AlertStatus::Active)
            .fold(HashMap::new(), |mut acc, alert| {
                *acc.entry(alert.level).or_insert(0) += 1;
                acc
            });

        AlertStats {
            total_active: active_by_level.values().sum(),
            active_critical: active_by_level
                .get(&AlertLevel::Critical)
                .copied()
                .unwrap_or(0),
            active_warning: active_by_level
                .get(&AlertLevel::Warning)
                .copied()
                .unwrap_or(0),
            active_info: active_by_level.get(&AlertLevel::Info).copied().unwrap_or(0),
            active_emergency: active_by_level
                .get(&AlertLevel::Emergency)
                .copied()
                .unwrap_or(0),
            total_resolved_today: history
                .iter()
                .filter(|alert| {
                    alert.status == AlertStatus::Resolved
                        && alert
                            .resolved_at
                            .map_or(false, |t| (Utc::now() - t).num_days() < 1)
                })
                .count() as u32,
        }
    }

    /// Cleanup old resolved alerts
    pub async fn cleanup_old_alerts(&self, retention_days: i64) {
        let cutoff = Utc::now() - Duration::days(retention_days);
        let mut active_alerts = self.active_alerts.write().await;
        let mut history = self.alert_history.write().await;

        // Move resolved alerts older than retention to history and remove from active
        let mut to_remove = Vec::new();
        let mut moved_count = 0;

        for (id, alert) in active_alerts.iter() {
            if alert.status == AlertStatus::Resolved {
                if let Some(resolved_at) = alert.resolved_at {
                    if resolved_at < cutoff {
                        history.push(alert.clone());
                        to_remove.push(*id);
                        moved_count += 1;
                    }
                }
            }
        }

        for id in to_remove {
            active_alerts.remove(&id);
        }

        // Also cleanup history if it gets too large
        history.truncate(10000); // Keep last 10k historical alerts

        if moved_count > 0 {
            info!(
                moved_count = moved_count,
                retention_days = retention_days,
                "Cleaned up old resolved alerts"
            );
        }
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    pub total_active: u32,
    pub active_critical: u32,
    pub active_warning: u32,
    pub active_info: u32,
    pub active_emergency: u32,
    pub total_resolved_today: u32,
}

/// Create default alert rules for List Sync system
pub fn create_default_alert_rules() -> Vec<AlertRule> {
    vec![
        // Consecutive sync failures
        AlertRule {
            name: "consecutive_sync_failures".to_string(),
            level: AlertLevel::Warning,
            description: "Multiple consecutive sync failures detected".to_string(),
            labels: HashMap::from([("component".to_string(), "list_sync".to_string())]),
            condition: AlertCondition::ConsecutiveFailures {
                service: "*".to_string(), // Wildcard for any service
                count: 3,
            },
            threshold: 3.0,
            evaluation_window: Duration::minutes(15),
            rate_limit: Some(Duration::minutes(30)),
            auto_resolve: false,
            auto_resolve_after: None,
            enabled: true,
        },
        // Slow sync operations
        AlertRule {
            name: "slow_sync_operation".to_string(),
            level: AlertLevel::Warning,
            description: "Sync operation is taking longer than expected".to_string(),
            labels: HashMap::from([("component".to_string(), "list_sync".to_string())]),
            condition: AlertCondition::SlowSync {
                service: "*".to_string(),
                duration_seconds: 300.0, // 5 minutes
            },
            threshold: 300.0,
            evaluation_window: Duration::minutes(5),
            rate_limit: Some(Duration::minutes(15)),
            auto_resolve: true,
            auto_resolve_after: Some(Duration::minutes(30)),
            enabled: true,
        },
        // High rate limit hits
        AlertRule {
            name: "high_rate_limit_hits".to_string(),
            level: AlertLevel::Critical,
            description: "High number of rate limit hits detected".to_string(),
            labels: HashMap::from([("component".to_string(), "api".to_string())]),
            condition: AlertCondition::RateLimitHits {
                service: "*".to_string(),
                hits_per_hour: 10,
            },
            threshold: 10.0,
            evaluation_window: Duration::hours(1),
            rate_limit: Some(Duration::hours(2)),
            auto_resolve: true,
            auto_resolve_after: Some(Duration::hours(4)),
            enabled: true,
        },
        // Service down
        AlertRule {
            name: "external_service_down".to_string(),
            level: AlertLevel::Critical,
            description: "External service is not responding".to_string(),
            labels: HashMap::from([("component".to_string(), "external_service".to_string())]),
            condition: AlertCondition::ServiceDown {
                service: "*".to_string(),
                check_interval_seconds: 300, // 5 minutes
            },
            threshold: 1.0,
            evaluation_window: Duration::minutes(5),
            rate_limit: Some(Duration::minutes(15)),
            auto_resolve: true,
            auto_resolve_after: None, // Auto-resolve immediately when service comes back
            enabled: true,
        },
        // Circuit breaker open
        AlertRule {
            name: "circuit_breaker_open".to_string(),
            level: AlertLevel::Warning,
            description: "Circuit breaker is open due to service failures".to_string(),
            labels: HashMap::from([("component".to_string(), "circuit_breaker".to_string())]),
            condition: AlertCondition::CircuitBreakerOpen {
                service: "*".to_string(),
            },
            threshold: 1.0,
            evaluation_window: Duration::minutes(1),
            rate_limit: Some(Duration::minutes(10)),
            auto_resolve: true,
            auto_resolve_after: None, // Auto-resolve when circuit closes
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_alert_creation() {
        let rule = AlertRule {
            name: "test_rule".to_string(),
            level: AlertLevel::Warning,
            description: "Test rule".to_string(),
            labels: HashMap::new(),
            condition: AlertCondition::ConsecutiveFailures {
                service: "test".to_string(),
                count: 3,
            },
            threshold: 3.0,
            evaluation_window: Duration::minutes(15),
            rate_limit: Some(Duration::minutes(30)),
            auto_resolve: false,
            auto_resolve_after: None,
            enabled: true,
        };

        let alert = Alert::new(&rule, "test_service", "Test Alert", "Test description");

        assert_eq!(alert.rule_name, "test_rule");
        assert_eq!(alert.level, AlertLevel::Warning);
        assert_eq!(alert.status, AlertStatus::Active);
        assert_eq!(alert.service, "test_service");
        assert_eq!(alert.fire_count, 1);
    }

    #[tokio::test]
    async fn test_alert_manager_consecutive_failures() {
        let manager = AlertManager::new();

        let rule = AlertRule {
            name: "consecutive_failures".to_string(),
            level: AlertLevel::Warning,
            description: "Test consecutive failures".to_string(),
            labels: HashMap::new(),
            condition: AlertCondition::ConsecutiveFailures {
                service: "imdb".to_string(),
                count: 3,
            },
            threshold: 3.0,
            evaluation_window: Duration::minutes(15),
            rate_limit: None,
            auto_resolve: false,
            auto_resolve_after: None,
            enabled: true,
        };

        manager.add_rule(rule).await.unwrap();

        // Should not trigger alert yet
        manager.check_consecutive_failures("imdb", 2).await;
        let alerts = manager.get_active_alerts().await;
        assert_eq!(alerts.len(), 0);

        // Should trigger alert
        manager.check_consecutive_failures("imdb", 3).await;
        let alerts = manager.get_active_alerts().await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].service, "imdb");
        assert_eq!(alerts[0].level, AlertLevel::Warning);
    }

    #[tokio::test]
    async fn test_alert_acknowledgment() {
        let manager = AlertManager::new();

        let rule = AlertRule {
            name: "test_rule".to_string(),
            level: AlertLevel::Warning,
            description: "Test rule".to_string(),
            labels: HashMap::new(),
            condition: AlertCondition::ConsecutiveFailures {
                service: "test".to_string(),
                count: 1,
            },
            threshold: 1.0,
            evaluation_window: Duration::minutes(15),
            rate_limit: None,
            auto_resolve: false,
            auto_resolve_after: None,
            enabled: true,
        };

        manager.add_rule(rule).await.unwrap();
        manager.check_consecutive_failures("test", 1).await;

        let alerts = manager.get_active_alerts().await;
        assert_eq!(alerts.len(), 1);
        let alert_id = alerts[0].id;

        // Acknowledge the alert
        manager
            .acknowledge_alert(alert_id, "test_user".to_string())
            .await
            .unwrap();

        // Should not appear in active alerts anymore
        let active_alerts = manager.get_active_alerts().await;
        assert_eq!(active_alerts.len(), 0);
    }
}
