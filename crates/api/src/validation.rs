//! Input validation middleware and utilities for the Radarr API
//!
//! This module provides comprehensive input validation using the validator crate
//! to prevent injection attacks and ensure data integrity.

use axum::{
    extract::{rejection::JsonRejection, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use validator::{Validate, ValidationError, ValidationErrors};
use std::collections::HashMap;

/// Custom validation error response
#[derive(Debug, serde::Serialize)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub details: HashMap<String, Vec<String>>,
}

impl IntoResponse for ValidationErrorResponse {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&self).unwrap_or_else(|_| {
            r#"{"error":"Validation failed","details":{}}"#.to_string()
        });
        
        (StatusCode::BAD_REQUEST, body).into_response()
    }
}

/// Convert validation errors to our custom error response
impl From<ValidationErrors> for ValidationErrorResponse {
    fn from(errors: ValidationErrors) -> Self {
        let mut details = HashMap::new();
        
        for (field, field_errors) in errors.field_errors() {
            let error_messages: Vec<String> = field_errors
                .iter()
                .map(|error| match &error.message {
                    Some(msg) => msg.to_string(),
                    None => format!("Invalid value for field '{}'", field),
                })
                .collect();
            details.insert(field.to_string(), error_messages);
        }
        
        Self {
            error: "Validation failed".to_string(),
            details,
        }
    }
}

/// Validated JSON extractor that automatically validates incoming JSON
pub async fn validate_json<T>(
    payload: Result<Json<T>, JsonRejection>,
) -> Result<Json<T>, ValidationErrorResponse>
where
    T: Validate + Send,
{
    match payload {
        Ok(Json(data)) => {
            // Validate the data
            data.validate()
                .map_err(ValidationErrorResponse::from)?;
            Ok(Json(data))
        }
        Err(rejection) => {
            // Handle JSON parsing errors
            let error_message = match rejection {
                JsonRejection::JsonDataError(_) => "Invalid JSON format",
                JsonRejection::JsonSyntaxError(_) => "Invalid JSON syntax", 
                JsonRejection::MissingJsonContentType(_) => "Missing Content-Type: application/json header",
                _ => "Invalid request body",
            };
            
            Err(ValidationErrorResponse {
                error: error_message.to_string(),
                details: HashMap::new(),
            })
        }
    }
}

/// Custom validators for Radarr-specific validation rules

/// Validate TMDB ID (positive integer)
pub fn validate_tmdb_id(id: &i32) -> Result<(), ValidationError> {
    if *id > 0 {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_tmdb_id"))
    }
}

/// Validate file path (no directory traversal)
pub fn validate_file_path(path: &str) -> Result<(), ValidationError> {
    if path.contains("..") || path.contains('\0') {
        return Err(ValidationError::new("invalid_file_path"));
    }
    
    // Additional checks for common injection patterns
    let dangerous_patterns = ["../", "..\\", "/etc/", "/var/", "/usr/", "C:\\", "\\\\"];
    for pattern in &dangerous_patterns {
        if path.contains(pattern) {
            return Err(ValidationError::new("suspicious_file_path"));
        }
    }
    
    Ok(())
}

/// Validate quality profile name (alphanumeric and basic punctuation only)
pub fn validate_quality_profile_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() || name.len() > 100 {
        return Err(ValidationError::new("invalid_length"));
    }
    
    // Allow alphanumeric, spaces, hyphens, and underscores
    let is_valid = name.chars().all(|c| {
        c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.'
    });
    
    if is_valid {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_characters"))
    }
}

/// Validate search query (prevent injection attacks)
pub fn validate_search_query(query: &str) -> Result<(), ValidationError> {
    if query.is_empty() || query.len() > 200 {
        return Err(ValidationError::new("invalid_length"));
    }
    
    // Check for suspicious patterns that might indicate injection attempts
    let suspicious_patterns = [
        "script>", "<script", "javascript:", "data:", "vbscript:", 
        "onload=", "onerror=", "onclick=", "eval(", "expression(",
        "SELECT ", "INSERT ", "UPDATE ", "DELETE ", "DROP ", "UNION ",
        "exec(", "system(", "cmd.exe", "/bin/sh", "bash", "powershell"
    ];
    
    let query_lower = query.to_lowercase();
    for pattern in &suspicious_patterns {
        if query_lower.contains(&pattern.to_lowercase()) {
            tracing::warn!("Suspicious search query detected: {}", query);
            return Err(ValidationError::new("suspicious_content"));
        }
    }
    
    Ok(())
}

/// Validate URL (must be HTTP/HTTPS)
pub fn validate_url(url: &str) -> Result<(), ValidationError> {
    if url.is_empty() || url.len() > 2048 {
        return Err(ValidationError::new("invalid_length"));
    }
    
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ValidationError::new("invalid_protocol"));
    }
    
    // Basic URL format validation
    if let Err(_) = url::Url::parse(url) {
        return Err(ValidationError::new("invalid_url_format"));
    }
    
    Ok(())
}

/// Validate API key format (must be alphanumeric and of reasonable length)
pub fn validate_api_key(key: &str) -> Result<(), ValidationError> {
    if key.len() < 16 || key.len() > 128 {
        return Err(ValidationError::new("invalid_api_key_length"));
    }
    
    let is_valid = key.chars().all(|c| c.is_alphanumeric());
    if !is_valid {
        return Err(ValidationError::new("invalid_api_key_format"));
    }
    
    Ok(())
}

/// Example validation struct for movie requests
#[derive(Debug, Deserialize, Validate)]
pub struct CreateMovieRequest {
    #[validate(range(min = 1))]
    pub tmdb_id: i32,
    
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    
    #[validate(range(min = 1900, max = 2100))]
    pub year: Option<i32>,
    
    #[validate(length(min = 1, max = 100))]
    pub quality_profile: Option<String>,
    
    #[validate(custom(function = "validate_file_path"))]
    pub root_folder: Option<String>,
}

/// Example validation struct for search requests
#[derive(Debug, Deserialize, Validate)]
pub struct SearchRequest {
    #[validate(custom(function = "validate_search_query"))]
    pub query: String,
    
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
    
    #[validate(range(min = 0))]
    pub offset: Option<u32>,
}

/// Example validation struct for configuration updates
#[derive(Debug, Deserialize, Validate)]
pub struct ConfigUpdateRequest {
    #[validate(custom(function = "validate_url"))]
    pub base_url: Option<String>,
    
    #[validate(custom(function = "validate_api_key"))]
    pub api_key: Option<String>,
    
    #[validate(range(min = 1, max = 3600))]
    pub timeout_seconds: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tmdb_id() {
        assert!(validate_tmdb_id(&12345).is_ok());
        assert!(validate_tmdb_id(&0).is_err());
        assert!(validate_tmdb_id(&-1).is_err());
    }

    #[test]
    fn test_validate_file_path() {
        assert!(validate_file_path("/safe/path/movie.mkv").is_ok());
        assert!(validate_file_path("movie.mkv").is_ok());
        assert!(validate_file_path("../../../etc/passwd").is_err());
        assert!(validate_file_path("/etc/passwd").is_err());
        assert!(validate_file_path("C:\\Windows\\System32").is_err());
    }

    #[test]
    fn test_validate_quality_profile_name() {
        assert!(validate_quality_profile_name("Ultra HD").is_ok());
        assert!(validate_quality_profile_name("HD-1080p").is_ok());
        assert!(validate_quality_profile_name("4K_HDR").is_ok());
        assert!(validate_quality_profile_name("").is_err());
        assert!(validate_quality_profile_name("<script>alert('xss')</script>").is_err());
    }

    #[test]
    fn test_validate_search_query() {
        assert!(validate_search_query("The Matrix").is_ok());
        assert!(validate_search_query("Star Wars: Episode IV").is_ok());
        assert!(validate_search_query("<script>alert('xss')</script>").is_err());
        assert!(validate_search_query("'; DROP TABLE movies; --").is_err());
        assert!(validate_search_query("").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://api.themoviedb.org").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("ftp://malicious.com").is_err());
        assert!(validate_url("javascript:alert('xss')").is_err());
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_validate_api_key() {
        assert!(validate_api_key("a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6").is_ok());
        assert!(validate_api_key("12345").is_err()); // too short
        assert!(validate_api_key("key-with-special-chars!").is_err());
    }

    #[test]
    fn test_create_movie_request_validation() {
        let valid_request = CreateMovieRequest {
            tmdb_id: 550,
            title: "Fight Club".to_string(),
            year: Some(1999),
            quality_profile: Some("Ultra HD".to_string()),
            root_folder: Some("/movies".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateMovieRequest {
            tmdb_id: -1, // Invalid TMDB ID
            title: "".to_string(), // Empty title
            year: Some(1800), // Invalid year
            quality_profile: Some("<script>".to_string()), // Invalid characters
            root_folder: Some("../../../etc/passwd".to_string()), // Path traversal
        };
        assert!(invalid_request.validate().is_err());
    }
}