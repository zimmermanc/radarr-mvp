//! Request extractors and validation

use crate::{
    error::{ApiError, ApiResult},
    models::PaginationParams,
};
use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use serde::de::DeserializeOwned;

/// Validated path extractor that ensures valid UUIDs
pub struct ValidatedPath<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::ValidationError {
                field: "path".to_string(),
                message: "Invalid path parameter".to_string(),
            })?;

        Ok(ValidatedPath(path.0))
    }
}

/// Validated pagination extractor
pub struct ValidatedPagination(pub PaginationParams);

#[async_trait]
impl<S> FromRequestParts<S> for ValidatedPagination
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let query_string = parts.uri.query().unwrap_or("");
        let params: PaginationParams =
            serde_urlencoded::from_str(query_string).map_err(|_| ApiError::ValidationError {
                field: "pagination".to_string(),
                message: "Invalid pagination parameters".to_string(),
            })?;

        // Validate pagination parameters
        if params.page == 0 {
            return Err(ApiError::ValidationError {
                field: "page".to_string(),
                message: "Page number must be >= 1".to_string(),
            });
        }

        if params.page_size == 0 || params.page_size > 1000 {
            return Err(ApiError::ValidationError {
                field: "page_size".to_string(),
                message: "Page size must be between 1 and 1000".to_string(),
            });
        }

        Ok(ValidatedPagination(params))
    }
}

/// Validate TMDB ID
pub fn validate_tmdb_id(tmdb_id: i32) -> ApiResult<()> {
    if tmdb_id <= 0 {
        return Err(ApiError::ValidationError {
            field: "tmdb_id".to_string(),
            message: "TMDB ID must be a positive integer".to_string(),
        });
    }
    Ok(())
}

/// Validate movie title
pub fn validate_movie_title(title: &str) -> ApiResult<()> {
    if title.trim().is_empty() {
        return Err(ApiError::ValidationError {
            field: "title".to_string(),
            message: "Movie title cannot be empty".to_string(),
        });
    }

    if title.len() > 500 {
        return Err(ApiError::ValidationError {
            field: "title".to_string(),
            message: "Movie title cannot exceed 500 characters".to_string(),
        });
    }

    Ok(())
}
