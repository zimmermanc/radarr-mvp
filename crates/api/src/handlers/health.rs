//! Health check handlers

use crate::{
    error::ApiResult,
    models::HealthResponse,
};
use axum::Json;
use serde_json::json;

/// GET /health - Basic health check
pub async fn health_check() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": "1.0.0"
    })))
}