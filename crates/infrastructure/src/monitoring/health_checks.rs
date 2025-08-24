//! Health check system for external list services
//!
//! This module provides health monitoring for external list services
//! including IMDb, TMDb, Trakt, and Plex integrations.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::sync::RwLock;
use tokio::time::{interval, timeout, Duration as TokioDuration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Health status of a service
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Service is healthy and responding
    Healthy,
    /// Service is experiencing issues but partially functional
    Degraded,
    /// Service is completely unavailable
    Unhealthy,
    /// Health status is unknown (e.g., never checked)
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }

    /// Get numeric score for aggregation (higher = better)
    pub fn score(&self) -> u8 {
        match self {
            Self::Healthy => 100,
            Self::Degraded => 50,
            Self::Unhealthy => 0,
            Self::Unknown => 25,
        }
    }
}

/// Health check result for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub service_name: String,
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub last_checked: DateTime<Utc>,
    pub last_healthy: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ServiceHealth {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            status: HealthStatus::Unknown,
            response_time_ms: None,
            last_checked: Utc::now(),
            last_healthy: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Update health status with check result
    pub fn update(
        &mut self,
        status: HealthStatus,
        response_time: Option<StdDuration>,
        error: Option<String>,
    ) {
        self.last_checked = Utc::now();
        self.response_time_ms = response_time.map(|d| d.as_millis() as u64);
        self.error_message = error;

        match status {
            HealthStatus::Healthy => {
                self.consecutive_successes += 1;
                self.consecutive_failures = 0;
                self.last_healthy = Some(Utc::now());
            }
            _ => {
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;
            }
        }

        self.status = status;
    }

    /// Check if service should be considered unhealthy based on failure count
    pub fn is_unhealthy_by_failures(&self, threshold: u32) -> bool {
        self.consecutive_failures >= threshold
    }

    /// Get time since last healthy check
    pub fn time_since_healthy(&self) -> Option<Duration> {
        self.last_healthy.map(|t| Utc::now() - t)
    }
}

/// Configuration for health checks
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// How often to run health checks
    pub check_interval: TokioDuration,
    /// Timeout for individual health check requests
    pub request_timeout: TokioDuration,
    /// Number of consecutive failures before marking as unhealthy
    pub failure_threshold: u32,
    /// Number of consecutive successes needed to recover from unhealthy
    pub recovery_threshold: u32,
    /// Whether to enable detailed health checks (may use more API calls)
    pub enable_detailed_checks: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval: TokioDuration::from_secs(300), // 5 minutes
            request_timeout: TokioDuration::from_secs(30),
            failure_threshold: 3,
            recovery_threshold: 2,
            enable_detailed_checks: false,
        }
    }
}

/// Trait for service-specific health check implementations
#[async_trait::async_trait]
pub trait ServiceHealthChecker: Send + Sync {
    /// Perform a health check for the service
    async fn check_health(&self) -> HealthCheckResult;

    /// Get the service name
    fn service_name(&self) -> &str;

    /// Check if the service supports detailed health checks
    fn supports_detailed_check(&self) -> bool {
        false
    }

    /// Perform a detailed health check (optional)
    async fn detailed_health_check(&self) -> HealthCheckResult {
        self.check_health().await
    }
}

/// Result of a health check operation
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub response_time: Option<StdDuration>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HealthCheckResult {
    pub fn healthy(response_time: StdDuration) -> Self {
        Self {
            status: HealthStatus::Healthy,
            response_time: Some(response_time),
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    pub fn degraded(response_time: StdDuration, message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            response_time: Some(response_time),
            error_message: Some(message.into()),
            metadata: HashMap::new(),
        }
    }

    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            response_time: None,
            error_message: Some(error.into()),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Main health checker that manages all service health checks
pub struct HealthChecker {
    config: HealthCheckConfig,
    checkers: Vec<Box<dyn ServiceHealthChecker>>,
    health_status: Arc<RwLock<HashMap<String, ServiceHealth>>>,
}

impl HealthChecker {
    /// Create a new health checker with configuration
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            checkers: Vec::new(),
            health_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a service health checker
    pub fn add_checker(&mut self, checker: Box<dyn ServiceHealthChecker>) {
        let service_name = checker.service_name().to_string();
        info!("Added health checker for service: {}", service_name);

        // Initialize health status
        let health_status = self.health_status.clone();
        tokio::spawn(async move {
            let mut status_map = health_status.write().await;
            status_map.insert(service_name.clone(), ServiceHealth::new(service_name));
        });

        self.checkers.push(checker);
    }

    /// Start the health check loop
    pub async fn start(&self) {
        info!(
            interval_secs = self.config.check_interval.as_secs(),
            timeout_secs = self.config.request_timeout.as_secs(),
            "Starting health check loop"
        );

        let mut interval = interval(self.config.check_interval);

        loop {
            interval.tick().await;

            // Run health checks for all services in parallel
            let check_futures: Vec<_> = self
                .checkers
                .iter()
                .map(|checker| self.check_service_health(checker.as_ref()))
                .collect();

            // Wait for all checks to complete
            futures::future::join_all(check_futures).await;

            debug!("Completed health check cycle");
        }
    }

    /// Perform health check for a specific service
    async fn check_service_health(&self, checker: &dyn ServiceHealthChecker) {
        let service_name = checker.service_name();
        let start_time = std::time::Instant::now();

        debug!("Checking health for service: {}", service_name);

        let result = timeout(self.config.request_timeout, async {
            if self.config.enable_detailed_checks && checker.supports_detailed_check() {
                checker.detailed_health_check().await
            } else {
                checker.check_health().await
            }
        })
        .await;

        let check_result = match result {
            Ok(check_result) => check_result,
            Err(_) => HealthCheckResult::unhealthy("Health check timed out"),
        };

        let elapsed = start_time.elapsed();

        // Update service health status
        let mut health_status = self.health_status.write().await;
        if let Some(service_health) = health_status.get_mut(service_name) {
            let old_status = service_health.status;
            service_health.update(
                check_result.status,
                check_result.response_time.or(Some(elapsed)),
                check_result.error_message.clone(),
            );
            service_health.metadata = check_result.metadata;

            // Log status changes
            if old_status != check_result.status {
                match check_result.status {
                    HealthStatus::Healthy => {
                        info!(
                            service = service_name,
                            response_time_ms = elapsed.as_millis(),
                            "Service health recovered"
                        );
                    }
                    HealthStatus::Degraded => {
                        warn!(
                            service = service_name,
                            error = check_result.error_message.as_deref().unwrap_or("Unknown"),
                            "Service health degraded"
                        );
                    }
                    HealthStatus::Unhealthy => {
                        error!(
                            service = service_name,
                            error = check_result.error_message.as_deref().unwrap_or("Unknown"),
                            consecutive_failures = service_health.consecutive_failures,
                            "Service unhealthy"
                        );
                    }
                    HealthStatus::Unknown => {
                        warn!(service = service_name, "Service health unknown");
                    }
                }
            } else {
                debug!(
                    service = service_name,
                    status = check_result.status.as_str(),
                    response_time_ms = elapsed.as_millis(),
                    "Health check completed"
                );
            }
        }
    }

    /// Get health status for all services
    pub async fn get_all_health_status(&self) -> HashMap<String, ServiceHealth> {
        self.health_status.read().await.clone()
    }

    /// Get health status for a specific service
    pub async fn get_service_health(&self, service_name: &str) -> Option<ServiceHealth> {
        let health_status = self.health_status.read().await;
        health_status.get(service_name).cloned()
    }

    /// Get overall health summary
    pub async fn get_health_summary(&self) -> HealthSummary {
        let health_status = self.health_status.read().await;
        let mut summary = HealthSummary::default();

        for (service_name, health) in health_status.iter() {
            summary.total_services += 1;

            match health.status {
                HealthStatus::Healthy => summary.healthy_services += 1,
                HealthStatus::Degraded => summary.degraded_services += 1,
                HealthStatus::Unhealthy => summary.unhealthy_services += 1,
                HealthStatus::Unknown => summary.unknown_services += 1,
            }

            if let Some(response_time) = health.response_time_ms {
                summary.avg_response_time_ms += response_time as f64;
            }

            if health.consecutive_failures > 0 {
                summary.services_with_issues.push(service_name.clone());
            }
        }

        if summary.total_services > 0 {
            summary.avg_response_time_ms /= summary.total_services as f64;
            summary.overall_health_score = ((summary.healthy_services * 100
                + summary.degraded_services * 50
                + summary.unknown_services * 25) as f64
                / summary.total_services as f64) as u8;
        }

        summary
    }

    /// Force a health check for a specific service
    pub async fn force_check(&self, service_name: &str) -> Option<HealthCheckResult> {
        if let Some(checker) = self
            .checkers
            .iter()
            .find(|c| c.service_name() == service_name)
        {
            let result = timeout(self.config.request_timeout, checker.check_health()).await;
            match result {
                Ok(check_result) => {
                    // Update the stored health status
                    let mut health_status = self.health_status.write().await;
                    if let Some(service_health) = health_status.get_mut(service_name) {
                        service_health.update(
                            check_result.status,
                            check_result.response_time,
                            check_result.error_message.clone(),
                        );
                    }
                    Some(check_result)
                }
                Err(_) => Some(HealthCheckResult::unhealthy("Health check timed out")),
            }
        } else {
            None
        }
    }
}

/// Health summary for all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_services: u32,
    pub healthy_services: u32,
    pub degraded_services: u32,
    pub unhealthy_services: u32,
    pub unknown_services: u32,
    pub overall_health_score: u8, // 0-100
    pub avg_response_time_ms: f64,
    pub services_with_issues: Vec<String>,
}

impl Default for HealthSummary {
    fn default() -> Self {
        Self {
            total_services: 0,
            healthy_services: 0,
            degraded_services: 0,
            unhealthy_services: 0,
            unknown_services: 0,
            overall_health_score: 0,
            avg_response_time_ms: 0.0,
            services_with_issues: Vec::new(),
        }
    }
}

/// HTTP-based health checker for services with REST APIs
pub struct HttpHealthChecker {
    service_name: String,
    url: String,
    client: reqwest::Client,
    expected_status: u16,
    timeout: StdDuration,
}

impl HttpHealthChecker {
    pub fn new(
        service_name: impl Into<String>,
        url: impl Into<String>,
        expected_status: u16,
        timeout: StdDuration,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            url: url.into(),
            client: reqwest::Client::new(),
            expected_status,
            timeout,
        }
    }
}

#[async_trait::async_trait]
impl ServiceHealthChecker for HttpHealthChecker {
    async fn check_health(&self) -> HealthCheckResult {
        let start_time = std::time::Instant::now();

        let response = match timeout(self.timeout, self.client.get(&self.url).send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                return HealthCheckResult::unhealthy(format!("HTTP request failed: {}", e))
            }
            Err(_) => return HealthCheckResult::unhealthy("Request timed out"),
        };

        let elapsed = start_time.elapsed();
        let status_code = response.status().as_u16();

        if status_code == self.expected_status {
            HealthCheckResult::healthy(elapsed)
                .with_metadata("status_code", serde_json::Value::Number(status_code.into()))
        } else if status_code >= 400 && status_code < 500 {
            HealthCheckResult::degraded(elapsed, format!("Unexpected status code: {}", status_code))
                .with_metadata("status_code", serde_json::Value::Number(status_code.into()))
        } else {
            HealthCheckResult::unhealthy(format!("HTTP error: {}", status_code))
        }
    }

    fn service_name(&self) -> &str {
        &self.service_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::time::sleep;

    struct MockHealthChecker {
        name: String,
        should_fail: Arc<AtomicBool>,
    }

    impl MockHealthChecker {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                should_fail: Arc::new(AtomicBool::new(false)),
            }
        }

        fn set_should_fail(&self, should_fail: bool) {
            self.should_fail.store(should_fail, Ordering::Relaxed);
        }
    }

    #[async_trait::async_trait]
    impl ServiceHealthChecker for MockHealthChecker {
        async fn check_health(&self) -> HealthCheckResult {
            sleep(TokioDuration::from_millis(10)).await; // Simulate network delay

            if self.should_fail.load(Ordering::Relaxed) {
                HealthCheckResult::unhealthy("Mock failure")
            } else {
                HealthCheckResult::healthy(StdDuration::from_millis(10))
            }
        }

        fn service_name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_service_health_update() {
        let mut health = ServiceHealth::new("test_service");

        // Initially unknown
        assert_eq!(health.status, HealthStatus::Unknown);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.consecutive_successes, 0);

        // Update with healthy status
        health.update(
            HealthStatus::Healthy,
            Some(StdDuration::from_millis(100)),
            None,
        );
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.consecutive_successes, 1);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_healthy.is_some());

        // Update with failure
        health.update(
            HealthStatus::Unhealthy,
            None,
            Some("Test error".to_string()),
        );
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.consecutive_failures, 1);
        assert_eq!(health.consecutive_successes, 0);
    }

    #[tokio::test]
    async fn test_health_checker_basic_functionality() {
        let config = HealthCheckConfig {
            check_interval: TokioDuration::from_millis(100),
            request_timeout: TokioDuration::from_secs(1),
            failure_threshold: 2,
            recovery_threshold: 1,
            enable_detailed_checks: false,
        };

        let mut health_checker = HealthChecker::new(config);
        let mock_checker = Box::new(MockHealthChecker::new("test_service"));

        health_checker.add_checker(mock_checker);

        // Give it a moment to initialize
        sleep(TokioDuration::from_millis(50)).await;

        // Force a health check
        let result = health_checker.force_check("test_service").await;
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!(result.status, HealthStatus::Healthy);

        // Check stored health status
        let health = health_checker.get_service_health("test_service").await;
        assert!(health.is_some());

        let health = health.unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.consecutive_successes, 1);
    }

    #[tokio::test]
    async fn test_health_summary() {
        let config = HealthCheckConfig::default();
        let mut health_checker = HealthChecker::new(config);

        // Add multiple mock checkers
        let mock1 = Box::new(MockHealthChecker::new("service1"));
        let mock2 = Box::new(MockHealthChecker::new("service2"));
        let mock3 = Box::new(MockHealthChecker::new("service3"));

        // Make service2 fail
        mock2.set_should_fail(true);

        health_checker.add_checker(mock1);
        health_checker.add_checker(mock2);
        health_checker.add_checker(mock3);

        // Give it time to initialize
        sleep(TokioDuration::from_millis(50)).await;

        // Force checks for all services
        health_checker.force_check("service1").await;
        health_checker.force_check("service2").await;
        health_checker.force_check("service3").await;

        let summary = health_checker.get_health_summary().await;
        assert_eq!(summary.total_services, 3);
        assert_eq!(summary.healthy_services, 2);
        assert_eq!(summary.unhealthy_services, 1);
        assert_eq!(summary.services_with_issues.len(), 1);
        assert!(summary
            .services_with_issues
            .contains(&"service2".to_string()));
    }
}
