//! API middleware

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};

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