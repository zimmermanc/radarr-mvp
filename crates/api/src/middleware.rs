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
    let path = request.uri().path();
    
    // Skip authentication for public endpoints
    if is_public_endpoint(path) {
        let response = next.run(request).await;
        return Ok(response);
    }
    
    // Get API key from various header options (Radarr compatibility)
    let api_key = headers
        .get("X-Api-Key")
        .or_else(|| headers.get("apikey"))
        .or_else(|| headers.get("ApiKey"))
        .and_then(|v| v.to_str().ok());
    
    // Get expected API key from environment - fail fast if not set
    let expected_api_key = std::env::var("RADARR_API_KEY")
        .expect("RADARR_API_KEY environment variable must be set for security");
    
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

/// Determine if an endpoint should be publicly accessible (no API key required)
fn is_public_endpoint(path: &str) -> bool {
    // Health check endpoints
    if path == "/health" || path == "/health/detailed" {
        return true;
    }
    
    // Static files and root paths (for React SPA)
    if path == "/" || path.starts_with("/static/") || path.starts_with("/assets/") {
        return true;
    }
    
    // Common static file extensions
    if path.ends_with(".html") || path.ends_with(".css") || path.ends_with(".js") 
       || path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg")
       || path.ends_with(".gif") || path.ends_with(".svg") || path.ends_with(".ico")
       || path.ends_with(".woff") || path.ends_with(".woff2") || path.ends_with(".ttf")
       || path.ends_with(".map") {
        return true;
    }
    
    // WebSocket endpoint handles its own authentication via query parameters
    if path == "/ws" {
        return true;
    }
    
    // Any other non-API paths (for SPA routing - React Router)
    if !path.starts_with("/api") && !path.starts_with("/metrics") {
        return true;
    }
    
    false
}