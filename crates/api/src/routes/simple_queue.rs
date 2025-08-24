//! Simple queue management routes

use axum::{
    routing::{delete, get, put},
    Router,
};

use crate::handlers::simple_queue::{
    list_queue_simple, pause_queue_item_simple, resume_queue_item_simple,
    remove_queue_item_simple, update_queue_priority_simple,
};

/// Create simplified queue routes
pub fn create_simple_queue_routes() -> Router<crate::handlers::simple_queue::SimpleQueueState> {
    Router::new()
        .route("/queue", get(list_queue_simple))
        .route("/queue/:id/pause", put(pause_queue_item_simple))
        .route("/queue/:id/resume", put(resume_queue_item_simple))
        .route("/queue/:id", delete(remove_queue_item_simple))
        .route("/queue/:id/priority", put(update_queue_priority_simple))
}