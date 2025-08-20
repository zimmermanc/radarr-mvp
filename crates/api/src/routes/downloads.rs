//! Download API routes

use crate::handlers::downloads;
use axum::{
    routing::{delete, get, post},
    Router,
};

/// Create download-related routes
pub fn create_download_routes() -> Router {
    Router::new()
        .route("/download", post(downloads::start_download))
        .route("/download", get(downloads::list_downloads))
        .route("/download/:id", get(downloads::get_download))
        .route("/download/:id", delete(downloads::cancel_download))
}