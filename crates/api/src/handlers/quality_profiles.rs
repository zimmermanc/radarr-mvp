//! Quality profile management API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::ApiResponse;
use radarr_infrastructure::DatabasePool;

/// Quality profile response structure
#[derive(Debug, Serialize)]
pub struct QualityProfile {
    pub id: i32,
    pub name: String,
    pub cutoff: i32,
    pub items: Vec<QualityItem>,
    pub min_format_score: i32,
    pub cutoff_format_score: i32,
    pub format_items: Vec<FormatItem>,
}

/// Quality item within a profile
#[derive(Debug, Serialize)]
pub struct QualityItem {
    pub quality: Quality,
    pub allowed: bool,
}

/// Quality definition
#[derive(Debug, Serialize)]
pub struct Quality {
    pub id: i32,
    pub name: String,
    pub source: String,
    pub resolution: i32,
}

/// Format item for custom formats
#[derive(Debug, Serialize)]
pub struct FormatItem {
    pub format: CustomFormat,
    pub name: String,
    pub score: i32,
}

/// Custom format definition
#[derive(Debug, Serialize)]
pub struct CustomFormat {
    pub id: i32,
    pub name: String,
    pub include_custom_format_when_renaming: bool,
}

/// State for quality profile handlers
#[derive(Clone)]
pub struct QualityProfileState {
    pub pool: DatabasePool,
}

impl QualityProfileState {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

/// GET /api/v3/qualityprofile - List all quality profiles
pub async fn list_quality_profiles(
    State(_state): State<QualityProfileState>,
) -> Json<ApiResponse<Vec<QualityProfile>>> {
    // For now, return default profiles until database implementation is ready
    let default_profiles = vec![
        QualityProfile {
            id: 1,
            name: "HD-1080p".to_string(),
            cutoff: 7,
            items: vec![
                QualityItem {
                    quality: Quality {
                        id: 1,
                        name: "SDTV".to_string(),
                        source: "Television".to_string(),
                        resolution: 480,
                    },
                    allowed: false,
                },
                QualityItem {
                    quality: Quality {
                        id: 2,
                        name: "DVD".to_string(),
                        source: "DVD".to_string(),
                        resolution: 480,
                    },
                    allowed: false,
                },
                QualityItem {
                    quality: Quality {
                        id: 4,
                        name: "HDTV-720p".to_string(),
                        source: "Television".to_string(),
                        resolution: 720,
                    },
                    allowed: true,
                },
                QualityItem {
                    quality: Quality {
                        id: 5,
                        name: "WEBDL-720p".to_string(),
                        source: "WebDL".to_string(),
                        resolution: 720,
                    },
                    allowed: true,
                },
                QualityItem {
                    quality: Quality {
                        id: 6,
                        name: "Bluray-720p".to_string(),
                        source: "BluRay".to_string(),
                        resolution: 720,
                    },
                    allowed: true,
                },
                QualityItem {
                    quality: Quality {
                        id: 7,
                        name: "WEBDL-1080p".to_string(),
                        source: "WebDL".to_string(),
                        resolution: 1080,
                    },
                    allowed: true,
                },
                QualityItem {
                    quality: Quality {
                        id: 8,
                        name: "Bluray-1080p".to_string(),
                        source: "BluRay".to_string(),
                        resolution: 1080,
                    },
                    allowed: true,
                },
            ],
            min_format_score: 0,
            cutoff_format_score: 0,
            format_items: vec![],
        },
        QualityProfile {
            id: 2,
            name: "Ultra-HD".to_string(),
            cutoff: 19,
            items: vec![
                QualityItem {
                    quality: Quality {
                        id: 18,
                        name: "WEBDL-2160p".to_string(),
                        source: "WebDL".to_string(),
                        resolution: 2160,
                    },
                    allowed: true,
                },
                QualityItem {
                    quality: Quality {
                        id: 19,
                        name: "Bluray-2160p".to_string(),
                        source: "BluRay".to_string(),
                        resolution: 2160,
                    },
                    allowed: true,
                },
            ],
            min_format_score: 0,
            cutoff_format_score: 0,
            format_items: vec![],
        },
    ];

    Json(ApiResponse::success(default_profiles))
}

/// GET /api/v3/qualityprofile/{id} - Get specific quality profile
pub async fn get_quality_profile(
    State(_state): State<QualityProfileState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<QualityProfile>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Return default profile based on ID
    let profile = match id {
        1 => QualityProfile {
            id: 1,
            name: "HD-1080p".to_string(),
            cutoff: 7,
            items: vec![], // Simplified for this response
            min_format_score: 0,
            cutoff_format_score: 0,
            format_items: vec![],
        },
        2 => QualityProfile {
            id: 2,
            name: "Ultra-HD".to_string(),
            cutoff: 19,
            items: vec![], // Simplified for this response
            min_format_score: 0,
            cutoff_format_score: 0,
            format_items: vec![],
        },
        _ => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Quality profile with ID {} not found",
                    id
                ))),
            ))
        }
    };

    Ok(Json(ApiResponse::success(profile)))
}