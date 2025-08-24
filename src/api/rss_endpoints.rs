//! RSS feed management API endpoints

use crate::services::RssService;
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Json,
};
use radarr_core::rss::{CalendarEntry, RssFeed};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Request to add a new RSS feed
#[derive(Debug, Deserialize)]
pub struct AddFeedRequest {
    pub name: String,
    pub url: String,
    pub interval_minutes: Option<u32>,
    pub categories: Option<Vec<String>>,
    pub quality_profile_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

/// Response for RSS feed
#[derive(Debug, Serialize)]
pub struct FeedResponse {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub interval_minutes: u32,
    pub last_check: Option<String>,
    pub last_sync: Option<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
}

/// Response for RSS test
#[derive(Debug, Serialize)]
pub struct TestFeedResponse {
    pub success: bool,
    pub items: Vec<RssItemResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RssItemResponse {
    pub title: String,
    pub url: String,
    pub pub_date: String,
    pub size: Option<u64>,
    pub category: Option<String>,
}

/// Calendar response
#[derive(Debug, Serialize)]
pub struct CalendarResponse {
    pub movie_id: Uuid,
    pub title: String,
    pub release_date: String,
    pub digital_release: Option<String>,
    pub physical_release: Option<String>,
    pub monitored: bool,
    pub days_until_search: Option<i64>,
}

/// Add a new RSS feed
pub async fn add_feed(
    Extension(rss_service): Extension<Arc<RssService>>,
    Json(request): Json<AddFeedRequest>,
) -> Result<(StatusCode, Json<FeedResponse>), StatusCode> {
    let mut feed = RssFeed::new(&request.name, &request.url);

    if let Some(interval) = request.interval_minutes {
        feed.interval_minutes = interval;
    }

    if let Some(categories) = request.categories {
        feed.categories = categories;
    }

    if let Some(profile_id) = request.quality_profile_id {
        feed.quality_profile_id = Some(profile_id);
    }

    if let Some(tags) = request.tags {
        feed.tags = tags;
    }

    let feed_id = feed.id;

    rss_service
        .add_feed(feed.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = FeedResponse {
        id: feed_id,
        name: feed.name,
        url: feed.url,
        enabled: feed.enabled,
        interval_minutes: feed.interval_minutes,
        last_check: feed.last_check.map(|dt| dt.to_rfc3339()),
        last_sync: feed.last_sync.map(|dt| dt.to_rfc3339()),
        categories: feed.categories,
        tags: feed.tags,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Remove an RSS feed
pub async fn remove_feed(
    Extension(rss_service): Extension<Arc<RssService>>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    match rss_service.remove_feed(id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Get all RSS feeds
pub async fn get_feeds(
    Extension(rss_service): Extension<Arc<RssService>>,
) -> Result<Json<Vec<FeedResponse>>, StatusCode> {
    let feeds = rss_service.get_feeds().await;

    let responses: Vec<FeedResponse> = feeds
        .into_iter()
        .map(|feed| FeedResponse {
            id: feed.id,
            name: feed.name,
            url: feed.url,
            enabled: feed.enabled,
            interval_minutes: feed.interval_minutes,
            last_check: feed.last_check.map(|dt| dt.to_rfc3339()),
            last_sync: feed.last_sync.map(|dt| dt.to_rfc3339()),
            categories: feed.categories,
            tags: feed.tags,
        })
        .collect();

    Ok(Json(responses))
}

/// Test an RSS feed
pub async fn test_feed(
    Extension(rss_service): Extension<Arc<RssService>>,
    Query(params): Query<TestFeedParams>,
) -> Result<Json<TestFeedResponse>, StatusCode> {
    let url = params.url.ok_or(StatusCode::BAD_REQUEST)?;

    match rss_service.test_feed(&url).await {
        Ok(items) => {
            let item_responses: Vec<RssItemResponse> = items
                .into_iter()
                .take(10) // Limit to 10 items for testing
                .map(|item| RssItemResponse {
                    title: item.title,
                    url: item.url,
                    pub_date: item.pub_date.to_rfc3339(),
                    size: item.size,
                    category: item.category,
                })
                .collect();

            Ok(Json(TestFeedResponse {
                success: true,
                items: item_responses,
                error: None,
            }))
        }
        Err(e) => Ok(Json(TestFeedResponse {
            success: false,
            items: Vec::new(),
            error: Some(e.to_string()),
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct TestFeedParams {
    pub url: Option<String>,
}

/// Get upcoming calendar entries
pub async fn get_calendar(
    Extension(rss_service): Extension<Arc<RssService>>,
    Query(params): Query<CalendarParams>,
) -> Result<Json<Vec<CalendarResponse>>, StatusCode> {
    let days = params.days.unwrap_or(30);
    let entries = rss_service.get_upcoming(days).await;

    let responses: Vec<CalendarResponse> = entries
        .into_iter()
        .map(|entry| {
            let days_until = entry
                .next_search_date()
                .map(|date| (date - chrono::Utc::now()).num_days());

            CalendarResponse {
                movie_id: entry.movie_id,
                title: entry.title,
                release_date: entry.release_date.to_rfc3339(),
                digital_release: entry.digital_release.map(|dt| dt.to_rfc3339()),
                physical_release: entry.physical_release.map(|dt| dt.to_rfc3339()),
                monitored: entry.monitored,
                days_until_search: days_until,
            }
        })
        .collect();

    Ok(Json(responses))
}

#[derive(Debug, Deserialize)]
pub struct CalendarParams {
    pub days: Option<i64>,
}

/// Add a calendar entry
pub async fn add_calendar_entry(
    Extension(rss_service): Extension<Arc<RssService>>,
    Json(entry): Json<CalendarEntry>,
) -> StatusCode {
    match rss_service.add_calendar_entry(entry).await {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
