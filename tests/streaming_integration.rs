//! Integration tests for streaming service endpoints

use radarr_core::streaming::{MediaType, TimeWindow, TrendingSource};
use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_trending_movies_endpoint() {
    let app = spawn_test_app().await;
    
    // Test trending movies endpoint
    let response = app
        .client
        .get(&format!("{}/api/streaming/trending/movie", &app.address))
        .header("X-Api-Key", &app.api_key)
        .query(&[("window", "day"), ("limit", "10")])
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["data"]["entries"].is_array());
    assert_eq!(body["data"]["window"], "day");
}

#[tokio::test]
async fn test_trending_tv_shows_endpoint() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .get(&format!("{}/api/streaming/trending/tv", &app.address))
        .header("X-Api-Key", &app.api_key)
        .query(&[("window", "week")])
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["data"]["entries"].is_array());
    assert_eq!(body["data"]["window"], "week");
}

#[tokio::test]
async fn test_availability_endpoint() {
    let app = spawn_test_app().await;
    
    // Test with a known TMDB ID (e.g., The Matrix = 603)
    let response = app
        .client
        .get(&format!("{}/api/streaming/availability/603", &app.address))
        .header("X-Api-Key", &app.api_key)
        .query(&[("region", "US")])
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["data"]["tmdb_id"], 603);
    assert_eq!(body["data"]["region"], "US");
    assert!(body["data"]["items"].is_array());
}

#[tokio::test]
async fn test_coming_soon_endpoint() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .get(&format!("{}/api/streaming/coming-soon/movie", &app.address))
        .header("X-Api-Key", &app.api_key)
        .query(&[("region", "US")])
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["data"]["media_type"], "movie");
    assert_eq!(body["data"]["region"], "US");
    assert!(body["data"]["releases"].is_array());
}

#[tokio::test]
async fn test_providers_endpoint() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .get(&format!("{}/api/streaming/providers", &app.address))
        .header("X-Api-Key", &app.api_key)
        .query(&[("region", "US")])
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["data"]["providers"].is_array());
    assert_eq!(body["data"]["region"], "US");
}

#[tokio::test]
async fn test_cache_refresh_endpoint() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .post(&format!("{}/api/streaming/cache/refresh", &app.address))
        .header("X-Api-Key", &app.api_key)
        .send()
        .await
        .expect("Failed to execute request");
    
    // Should return OK or ACCEPTED
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::ACCEPTED,
        "Expected OK or ACCEPTED, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_trakt_auth_init_endpoint() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .post(&format!("{}/api/streaming/trakt/auth/init", &app.address))
        .header("X-Api-Key", &app.api_key)
        .send()
        .await
        .expect("Failed to execute request");
    
    // May return OK if Trakt is configured, or an error if not
    if response.status() == StatusCode::OK {
        let body: serde_json::Value = response.json().await.expect("Failed to parse response");
        assert!(body["data"]["device_code"].is_string());
        assert!(body["data"]["user_code"].is_string());
        assert!(body["data"]["verification_url"].is_string());
    }
}

#[tokio::test]
async fn test_invalid_media_type() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .get(&format!("{}/api/streaming/trending/invalid", &app.address))
        .header("X-Api-Key", &app.api_key)
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_missing_api_key() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .get(&format!("{}/api/streaming/trending/movie", &app.address))
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_trending_with_source_filter() {
    let app = spawn_test_app().await;
    
    let response = app
        .client
        .get(&format!("{}/api/streaming/trending/movie", &app.address))
        .header("X-Api-Key", &app.api_key)
        .query(&[("source", "tmdb"), ("window", "day")])
        .send()
        .await
        .expect("Failed to execute request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["data"]["source"], "tmdb");
}