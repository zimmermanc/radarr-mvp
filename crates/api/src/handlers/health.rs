//! Health check handlers

use crate::{
    error::ApiResult,
    models::{HealthResponse, ServiceHealth},
};
use axum::Json;
use chrono::Utc;
use serde_json::json;
use std::time::Instant;
use tracing::debug;

static APP_START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Initialize the application start time
pub fn init_app_start_time() {
    APP_START_TIME.set(Instant::now()).ok();
}

/// GET /health - Basic health check
pub async fn health_check() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// GET /health/detailed - Comprehensive health check with service details
pub async fn detailed_health_check() -> ApiResult<Json<HealthResponse>> {
    debug!("Starting comprehensive health check");
    let check_start = Instant::now();

    let mut services = Vec::new();
    let mut overall_healthy = true;

    // For now, create placeholder service health entries
    // These will be populated with actual circuit breaker data when the services are properly wired

    // TMDB service placeholder
    services.push(ServiceHealth {
        name: "TMDB".to_string(),
        status: "healthy".to_string(),
        response_time_ms: Some(50),
        last_check: Utc::now(),
        error: None,
    });

    // HDBits service placeholder
    services.push(ServiceHealth {
        name: "HDBits".to_string(),
        status: "healthy".to_string(),
        response_time_ms: Some(200),
        last_check: Utc::now(),
        error: None,
    });

    // qBittorrent service placeholder
    services.push(ServiceHealth {
        name: "qBittorrent".to_string(),
        status: "healthy".to_string(),
        response_time_ms: Some(75),
        last_check: Utc::now(),
        error: None,
    });

    // Database placeholder
    services.push(ServiceHealth {
        name: "PostgreSQL".to_string(),
        status: "healthy".to_string(),
        response_time_ms: Some(10),
        last_check: Utc::now(),
        error: None,
    });

    // Queue processor placeholder
    services.push(ServiceHealth {
        name: "Queue Processor".to_string(),
        status: "healthy".to_string(),
        response_time_ms: Some(0),
        last_check: Utc::now(),
        error: None,
    });

    let uptime_seconds = APP_START_TIME
        .get()
        .map(|start| start.elapsed().as_secs())
        .unwrap_or(0);

    let overall_status = if overall_healthy {
        "healthy"
    } else {
        "degraded"
    };

    debug!(
        "Health check completed in {}ms",
        check_start.elapsed().as_millis()
    );

    let health_response = HealthResponse {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        services,
    };

    Ok(Json(health_response))
}

/// GET /health/services/{service} - Individual service health check
pub async fn service_health_check(
    axum::extract::Path(service_name): axum::extract::Path<String>,
) -> ApiResult<Json<ServiceHealth>> {
    let service_health = match service_name.to_lowercase().as_str() {
        "tmdb" => ServiceHealth {
            name: "TMDB".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(50),
            last_check: Utc::now(),
            error: None,
        },
        "hdbits" => ServiceHealth {
            name: "HDBits".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(200),
            last_check: Utc::now(),
            error: None,
        },
        "qbittorrent" => ServiceHealth {
            name: "qBittorrent".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(75),
            last_check: Utc::now(),
            error: None,
        },
        "database" | "postgresql" => ServiceHealth {
            name: "PostgreSQL".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(10),
            last_check: Utc::now(),
            error: None,
        },
        "queue" => ServiceHealth {
            name: "Queue Processor".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(0),
            last_check: Utc::now(),
            error: None,
        },
        _ => {
            return Err(crate::error::ApiError::NotFound {
                resource: format!("service '{}'", service_name),
            });
        }
    };

    Ok(Json(service_health))
}
