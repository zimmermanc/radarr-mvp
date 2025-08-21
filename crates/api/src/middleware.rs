//! API middleware

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, HeaderMap},
    middleware::Next,
    response::Response as AxumResponse,
};
use std::sync::Arc;

/// Simple request logging middleware
pub async fn request_logger(
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    let response = next.run(request).await;
    
    tracing::info!("{} {} -> {}", method, uri, response.status());
    
    response
}

/// API key authentication middleware
pub async fn require_api_key(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<AxumResponse, StatusCode> {
    // Skip authentication for health endpoint
    let path = request.uri().path();
    if path == "/health" {
        let response = next.run(request).await;
        return Ok(response);
    }
    
    // Get API key from various header options (Radarr compatibility)
    let api_key = headers
        .get("X-Api-Key")
        .or_else(|| headers.get("apikey"))
        .or_else(|| headers.get("ApiKey"))
        .and_then(|v| v.to_str().ok());
    
    // Get expected API key from environment or use default
    let expected_api_key = std::env::var("RADARR_API_KEY")
        .unwrap_or_else(|_| "changeme123".to_string());
    
    match api_key {
        Some(key) if key == expected_api_key => {
            let response = next.run(request).await;
            Ok(response)
        }
        Some(_) => {
            tracing::warn!("Invalid API key provided for {}", path);
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!("No API key provided for {}", path);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}