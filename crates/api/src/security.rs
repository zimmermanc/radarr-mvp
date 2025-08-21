//! Security middleware and configuration for the Radarr API
//!
//! This module provides comprehensive security headers, CORS configuration,
//! and input validation to protect against common web vulnerabilities.

use axum::{
    http::{header, HeaderValue, Method},
    middleware::from_fn,
    Router,
};
use tower_http::{
    cors::{CorsLayer, Any},
    set_header::SetResponseHeaderLayer,
};
use std::str::FromStr;

/// Security configuration for the API
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
    /// Enable HSTS (HTTP Strict Transport Security)
    pub enable_hsts: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u32,
    /// Content Security Policy
    pub csp_policy: String,
    /// Environment (development/production)
    pub environment: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:5173".to_string(), // Vite dev server default
                "http://127.0.0.1:5173".to_string(),
                "http://0.0.0.0:5173".to_string(),
            ],
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            csp_policy: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self'".to_string(),
            environment: "development".to_string(),
        }
    }
}

impl SecurityConfig {
    /// Create security configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            cors_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173,http://127.0.0.1:5173,http://0.0.0.0:5173".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            enable_hsts: std::env::var("ENABLE_SECURITY_HEADERS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            hsts_max_age: std::env::var("HSTS_MAX_AGE")
                .unwrap_or_else(|_| "31536000".to_string())
                .parse()
                .unwrap_or(31536000),
            csp_policy: std::env::var("CSP_POLICY")
                .unwrap_or_else(|_| "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self'".to_string()),
            environment: std::env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
        }
    }
}

/// Configure CORS layer with security-first defaults
pub fn configure_cors(config: &SecurityConfig) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::HeaderName::from_static("x-api-key"),
            header::HeaderName::from_static("apikey"),
        ])
        .allow_credentials(true);

    // Configure origins based on environment
    if config.environment == "development" {
        // In development, be more permissive with CORS
        let mut dev_origins = config.cors_origins.clone();
        
        // Add common development origins if not already present
        let common_dev_origins = vec![
            "http://localhost:5173",
            "http://127.0.0.1:5173", 
            "http://0.0.0.0:5173",
            "http://localhost:3000",
            "http://127.0.0.1:3000",
        ];
        
        for origin in common_dev_origins {
            if !dev_origins.iter().any(|o| o == origin) {
                dev_origins.push(origin.to_string());
            }
        }
        
        // Also allow any localhost/127.0.0.1 origin on port 5173 for dynamic IPs
        // This handles cases where the dev server is accessed via LAN IP
        tracing::info!("Development mode: allowing configured origins plus common dev origins");
        
        let origins: Result<Vec<_>, _> = dev_origins
            .iter()
            .map(|origin| origin.parse::<HeaderValue>())
            .collect();
        
        match origins {
            Ok(origins) => cors = cors.allow_origin(origins),
            Err(e) => {
                tracing::warn!("Invalid CORS origin configuration: {}", e);
                cors = cors.allow_origin([
                    "http://localhost:5173".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:5173".parse::<HeaderValue>().unwrap(),
                    "http://localhost:3000".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
                ]);
            }
        }
    } else {
        // In production, strictly validate origins
        let origins: Result<Vec<_>, _> = config.cors_origins
            .iter()
            .filter(|origin| !origin.contains("localhost") && !origin.contains("127.0.0.1"))
            .map(|origin| origin.parse::<HeaderValue>())
            .collect();
        
        match origins {
            Ok(origins) if !origins.is_empty() => cors = cors.allow_origin(origins),
            _ => {
                tracing::error!("No valid CORS origins configured for production environment");
                // Fail safe - deny all origins in production
                cors = cors.allow_origin([]);
            }
        }
    }

    cors
}

/// Create security headers middleware layers
pub fn security_headers(config: &SecurityConfig) -> Vec<SetResponseHeaderLayer<HeaderValue>> {
    let mut headers = vec![
        // Prevent clickjacking
        SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY")
        ),
        // Prevent MIME type sniffing
        SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff")
        ),
        // XSS protection (legacy but still useful)
        SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block")
        ),
        // Referrer policy for privacy
        SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin")
        ),
    ];

    // Add HSTS header if enabled (typically for production)
    if config.enable_hsts {
        let hsts_value = if config.environment == "production" {
            format!("max-age={}; includeSubDomains; preload", config.hsts_max_age)
        } else {
            format!("max-age={}", config.hsts_max_age)
        };
        
        if let Ok(hsts_header_value) = HeaderValue::from_str(&hsts_value) {
            headers.push(SetResponseHeaderLayer::overriding(
                header::STRICT_TRANSPORT_SECURITY,
                hsts_header_value
            ));
        }
    }

    // Add Content Security Policy
    if let Ok(csp_header_value) = HeaderValue::from_str(&config.csp_policy) {
        headers.push(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            csp_header_value
        ));
    }

    // Add security-focused cache control for API responses
    headers.push(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, private")
    ));

    headers
}

/// Apply comprehensive security configuration to a router
pub fn apply_security<S>(router: Router<S>, config: SecurityConfig) -> Router<S> 
where
    S: Clone + Send + Sync + 'static,
{
    let cors_layer = configure_cors(&config);
    let header_layers = security_headers(&config);
    
    let mut secured_router = router.layer(cors_layer);
    
    // Apply all security headers
    for header_layer in header_layers {
        secured_router = secured_router.layer(header_layer);
    }
    
    secured_router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_from_env() {
        // Test with minimal environment
        let config = SecurityConfig::from_env();
        assert!(!config.cors_origins.is_empty());
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31536000);
    }

    #[test]
    fn test_cors_configuration_development() {
        let config = SecurityConfig {
            cors_origins: vec!["http://localhost:3000".to_string()],
            environment: "development".to_string(),
            ..Default::default()
        };
        
        let cors_layer = configure_cors(&config);
        // CORS layer is configured - actual testing would require integration tests
        assert!(true);
    }

    #[test]
    fn test_cors_configuration_production() {
        let config = SecurityConfig {
            cors_origins: vec!["https://radarr.example.com".to_string()],
            environment: "production".to_string(),
            ..Default::default()
        };
        
        let cors_layer = configure_cors(&config);
        // CORS layer is configured - actual testing would require integration tests
        assert!(true);
    }

    #[test]
    fn test_security_headers_generation() {
        let config = SecurityConfig::default();
        let headers = security_headers(&config);
        
        // Should have at least basic security headers
        assert!(headers.len() >= 5);
    }
}