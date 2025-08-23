//! Quality profile and custom format API handlers

use crate::error::{ApiError, ApiResult};
use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use radarr_decision::{CustomFormat, FormatSpecification, CustomFormatEngine, ReleaseData};
use radarr_infrastructure::{DatabasePool, PostgresCustomFormatsRepository, CustomFormatsRepository};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, debug, instrument};
use uuid::Uuid;

/// Quality management state
#[derive(Clone)]
pub struct QualityState {
    pub database_pool: DatabasePool,
    pub custom_formats_repo: Arc<PostgresCustomFormatsRepository>,
}

impl QualityState {
    pub fn new(database_pool: DatabasePool) -> Self {
        let custom_formats_repo = Arc::new(PostgresCustomFormatsRepository::new(database_pool.clone()));
        Self {
            database_pool,
            custom_formats_repo,
        }
    }
}

/// Custom format request for API
#[derive(Debug, Deserialize)]
pub struct CustomFormatRequest {
    pub name: String,
    pub specifications: Vec<FormatSpecificationRequest>,
    pub score: i32,
    pub enabled: Option<bool>,
}

/// Format specification request for API
#[derive(Debug, Deserialize)]
pub struct CustomFormatSpecificationRequest {
    #[serde(rename = "type")]
    pub spec_type: String,
    pub negate: Option<bool>,
    pub required: Option<bool>,
    pub value: String,
}

/// Format specification request alias for backwards compatibility
pub type FormatSpecificationRequest = CustomFormatSpecificationRequest;

/// Custom format response for API
#[derive(Debug, Serialize)]
pub struct CustomFormatResponse {
    pub id: String,
    pub name: String,
    pub specifications: Vec<FormatSpecificationResponse>,
    pub score: i32,
    pub enabled: bool,
}

/// Format specification response for API
#[derive(Debug, Serialize)]
pub struct FormatSpecificationResponse {
    #[serde(rename = "type")]
    pub spec_type: String,
    pub negate: bool,
    pub required: bool,
    pub value: String,
}

impl From<CustomFormat> for CustomFormatResponse {
    fn from(format: CustomFormat) -> Self {
        Self {
            id: format.id.to_string(),
            name: format.name,
            specifications: format.specifications.into_iter().map(Into::into).collect(),
            score: format.score,
            enabled: format.enabled,
        }
    }
}

impl From<FormatSpecification> for FormatSpecificationResponse {
    fn from(spec: FormatSpecification) -> Self {
        Self {
            spec_type: spec.spec_type,
            negate: spec.negate,
            required: spec.required,
            value: spec.value,
        }
    }
}

impl From<CustomFormatRequest> for CustomFormat {
    fn from(request: CustomFormatRequest) -> Self {
        let specifications = request.specifications
            .into_iter()
            .map(|spec| FormatSpecification {
                spec_type: spec.spec_type,
                negate: spec.negate.unwrap_or(false),
                required: spec.required.unwrap_or(false),
                value: spec.value,
            })
            .collect();

        CustomFormat {
            id: Uuid::new_v4(),
            name: request.name,
            specifications,
            score: request.score,
            enabled: request.enabled.unwrap_or(true),
        }
    }
}

/// Test release against custom formats
#[derive(Debug, Deserialize)]
pub struct TestReleaseRequest {
    pub title: String,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,
    pub freeleech: Option<bool>,
    pub internal: Option<bool>,
    pub indexer: Option<String>,
}

/// Test release response
#[derive(Debug, Serialize)]
pub struct TestReleaseResponse {
    pub total_score: i32,
    pub matching_formats: Vec<MatchingFormatInfo>,
}

/// Information about a matching custom format
#[derive(Debug, Serialize)]
pub struct MatchingFormatInfo {
    pub name: String,
    pub score: i32,
    pub specifications_matched: Vec<String>,
}

/// GET /api/v3/customformat - List all custom formats
#[instrument(skip(state))]
pub async fn list_custom_formats(
    State(state): State<QualityState>,
) -> ApiResult<Json<Vec<CustomFormatResponse>>> {
    info!("Listing all custom formats");
    
    let formats = state.custom_formats_repo.list().await
        .map_err(ApiError::CoreError)?;
    
    let responses: Vec<CustomFormatResponse> = formats.into_iter().map(Into::into).collect();
    
    info!("Retrieved {} custom formats", responses.len());
    Ok(Json(responses))
}

/// GET /api/v3/customformat/:id - Get custom format by ID
#[instrument(skip(state))]
pub async fn get_custom_format(
    State(state): State<QualityState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CustomFormatResponse>> {
    let format_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid custom format ID".to_string(),
        })?;
    
    debug!("Getting custom format with ID: {}", format_id);
    
    let format = state.custom_formats_repo.find_by_id(&format_id).await
        .map_err(ApiError::CoreError)?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("custom format with id {}", id),
        })?;
    
    Ok(Json(format.into()))
}

/// POST /api/v3/customformat - Create new custom format
#[instrument(skip(state))]
pub async fn create_custom_format(
    State(state): State<QualityState>,
    Json(request): Json<CustomFormatRequest>,
) -> ApiResult<Json<CustomFormatResponse>> {
    info!("Creating custom format: {}", request.name);
    
    // Check if format with this name already exists
    if let Ok(Some(_)) = state.custom_formats_repo.find_by_name(&request.name).await {
        return Err(ApiError::Conflict {
            resource: format!("custom format '{}'", request.name),
        });
    }
    
    let format: CustomFormat = request.into();
    
    let created_format = state.custom_formats_repo.create(&format).await
        .map_err(ApiError::CoreError)?;
    
    info!("Created custom format '{}' with ID: {}", created_format.name, created_format.id);
    Ok(Json(created_format.into()))
}

/// PUT /api/v3/customformat/:id - Update custom format
#[instrument(skip(state))]
pub async fn update_custom_format(
    State(state): State<QualityState>,
    Path(id): Path<String>,
    Json(request): Json<CustomFormatRequest>,
) -> ApiResult<Json<CustomFormatResponse>> {
    let format_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid custom format ID".to_string(),
        })?;
    
    info!("Updating custom format: {} ({})", request.name, format_id);
    
    // Check if format exists
    if state.custom_formats_repo.find_by_id(&format_id).await
        .map_err(ApiError::CoreError)?.is_none() {
        return Err(ApiError::NotFound {
            resource: format!("custom format with id {}", id),
        });
    }
    
    let mut format: CustomFormat = request.into();
    format.id = format_id; // Preserve the existing ID
    
    let updated_format = state.custom_formats_repo.update(&format).await
        .map_err(ApiError::CoreError)?;
    
    info!("Updated custom format '{}'", updated_format.name);
    Ok(Json(updated_format.into()))
}

/// DELETE /api/v3/customformat/:id - Delete custom format
#[instrument(skip(state))]
pub async fn delete_custom_format(
    State(state): State<QualityState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let format_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid custom format ID".to_string(),
        })?;
    
    info!("Deleting custom format with ID: {}", format_id);
    
    // Check if format exists
    let format = state.custom_formats_repo.find_by_id(&format_id).await
        .map_err(ApiError::CoreError)?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("custom format with id {}", id),
        })?;
    
    state.custom_formats_repo.delete(&format_id).await
        .map_err(ApiError::CoreError)?;
    
    info!("Deleted custom format '{}'", format.name);
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v3/customformat/test - Test release against custom formats
#[instrument(skip(state))]
pub async fn test_custom_formats(
    State(state): State<QualityState>,
    Json(request): Json<TestReleaseRequest>,
) -> ApiResult<Json<TestReleaseResponse>> {
    info!("Testing release against custom formats: {}", request.title);
    
    // Load all enabled custom formats
    let formats = state.custom_formats_repo.list_enabled().await
        .map_err(ApiError::CoreError)?;
    
    // Create custom format engine
    let engine = CustomFormatEngine::with_formats(formats);
    
    // Create release data for testing
    let release_data = ReleaseData {
        title: request.title.clone(),
        size_bytes: request.size_bytes,
        seeders: request.seeders,
        leechers: None,
        freeleech: request.freeleech,
        internal: request.internal,
        indexer: request.indexer.unwrap_or_else(|| "Test".to_string()),
        release_group: request.title.split('-').last().map(|s| s.trim().to_string()),
    };
    
    // Calculate score and get matching formats
    let total_score = engine.calculate_format_score(&release_data);
    let matching_formats = engine.get_matching_formats(&release_data);
    
    let matching_info: Vec<MatchingFormatInfo> = matching_formats
        .into_iter()
        .map(|format| MatchingFormatInfo {
            name: format.name.clone(),
            score: format.score,
            specifications_matched: format.specifications
                .iter()
                .filter(|spec| spec.matches(&release_data))
                .map(|spec| format!("{}: {}", spec.spec_type, spec.value))
                .collect(),
        })
        .collect();
    
    let response = TestReleaseResponse {
        total_score,
        matching_formats: matching_info,
    };
    
    info!("Release test completed: total score = {}, {} formats matched", 
          total_score, response.matching_formats.len());
    
    Ok(Json(response))
}

/// Create quality management router
pub fn create_quality_router(state: QualityState) -> Router {
    Router::new()
        .route("/customformat", get(list_custom_formats).post(create_custom_format))
        .route("/customformat/:id", get(get_custom_format).put(update_custom_format).delete(delete_custom_format))
        .route("/customformat/test", post(test_custom_formats))
        .with_state(state)
}