//! Advanced Search Routes
//! Enhanced search and filtering API endpoints

use crate::handlers::advanced_search::{advanced_search, bulk_operations, AdvancedSearchState};
use axum::{
    routing::{get, post},
    Router,
};
use radarr_infrastructure::DatabasePool;

/// Create advanced search routes
pub fn create_advanced_search_routes(database_pool: DatabasePool) -> Router {
    let state = AdvancedSearchState::new(database_pool);

    Router::new()
        .route("/api/v3/search/advanced", get(advanced_search))
        .route("/api/v3/search/bulk", post(bulk_operations))
        .with_state(state)
}

// Tests would require axum_test dependency - commented out for now
/*
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use radarr_infrastructure::DatabasePool;

    #[tokio::test]
    async fn test_advanced_search_route() {
        let database_pool = DatabasePool::default();
        let app = create_advanced_search_routes(database_pool);
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/v3/search/advanced")
            .add_query_param("min_quality_score", "80")
            .add_query_param("freeleech_only", "true")
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let json: serde_json::Value = response.json();
        assert!(json.get("results").is_some());
        assert!(json.get("search_metadata").is_some());
        assert!(json.get("recommendations").is_some());
    }

    #[tokio::test]
    async fn test_bulk_operations_route() {
        let database_pool = DatabasePool::default();
        let app = create_advanced_search_routes(database_pool);
        let server = TestServer::new(app).unwrap();

        let bulk_request = serde_json::json!({
            "operation": "download",
            "release_guids": ["test-guid-1", "test-guid-2"],
            "target_quality": "1080p"
        });

        let response = server
            .post("/api/v3/search/bulk")
            .json(&bulk_request)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let json: serde_json::Value = response.json();
        assert!(json.get("successful").is_some());
        assert!(json.get("total_items").is_some());
        assert_eq!(json["total_items"], 2);
    }
}
*/
