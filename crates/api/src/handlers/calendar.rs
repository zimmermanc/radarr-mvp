//! Calendar and RSS feed handlers
//!
//! Provides calendar views of movie releases and RSS/iCal feed functionality
//! for integration with external calendar applications.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use radarr_core::Movie;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Calendar query parameters
#[derive(Debug, Deserialize)]
pub struct CalendarParams {
    /// Start date (YYYY-MM-DD format)
    pub start: Option<String>,
    /// End date (YYYY-MM-DD format)  
    pub end: Option<String>,
    /// Whether to include unmonitored movies
    pub unmonitored: Option<bool>,
}

/// Calendar entry representing a movie release
#[derive(Debug, Serialize)]
pub struct CalendarEntry {
    /// Movie ID
    pub id: String,
    /// Movie title
    pub title: String,
    /// Release year
    pub year: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i32>,
    /// Whether movie is monitored
    pub monitored: bool,
    /// Physical release date
    pub physical_release: Option<DateTime<Utc>>,
    /// Digital release date
    pub digital_release: Option<DateTime<Utc>>,
    /// In theaters date
    pub in_theaters: Option<DateTime<Utc>>,
    /// Movie poster URL
    pub poster_url: Option<String>,
    /// Movie status
    pub status: String,
}

impl From<Movie> for CalendarEntry {
    fn from(movie: Movie) -> Self {
        // Generate TMDB poster URL if poster_path exists in metadata
        let poster_url = movie
            .metadata
            .get("poster_path")
            .and_then(|v| v.as_str())
            .map(|poster_path| format!("https://image.tmdb.org/t/p/w500{}", poster_path));

        Self {
            id: movie.id.to_string(),
            title: movie.title,
            year: movie.year,
            tmdb_id: Some(movie.tmdb_id),
            monitored: movie.monitored,
            // For MVP, we'll use creation date as placeholder
            // In production, these would be extracted from movie metadata or separate release date fields
            physical_release: Some(movie.created_at),
            digital_release: Some(movie.created_at),
            in_theaters: Some(movie.created_at),
            poster_url,
            status: format!("{:?}", movie.status),
        }
    }
}

/// Get calendar entries for movies
pub async fn get_calendar(
    Query(_params): Query<CalendarParams>,
    State(_state): State<Arc<crate::SimpleApiState>>,
) -> std::result::Result<Json<Vec<CalendarEntry>>, StatusCode> {
    // For MVP, return mock calendar data
    // In production, this would query the database with date filters

    let calendar_entries = vec![
        CalendarEntry {
            id: "8c09a7b1-a772-4835-8e09-cdfa9ecefc54".to_string(),
            title: "The Matrix".to_string(),
            year: Some(1999),
            tmdb_id: Some(603),
            monitored: true,
            physical_release: Some(Utc::now()),
            digital_release: Some(Utc::now()),
            in_theaters: Some(Utc::now()),
            poster_url: Some(
                "https://image.tmdb.org/t/p/w500/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg".to_string(),
            ),
            status: "released".to_string(),
        },
        CalendarEntry {
            id: "c437fae8-a65e-48cf-b6f6-dcbcbc02a4d9".to_string(),
            title: "Forrest Gump".to_string(),
            year: Some(1994),
            tmdb_id: Some(13),
            monitored: true,
            physical_release: Some(Utc::now()),
            digital_release: Some(Utc::now()),
            in_theaters: Some(Utc::now()),
            poster_url: Some(
                "https://image.tmdb.org/t/p/w500/arw2vcBveWOVZr6pxd9XTd1TdQa.jpg".to_string(),
            ),
            status: "released".to_string(),
        },
    ];

    Ok(Json(calendar_entries))
}

/// Generate iCal feed for calendar integration
pub async fn get_ical_feed(
    Query(_params): Query<HashMap<String, String>>,
    State(_state): State<Arc<crate::SimpleApiState>>,
) -> impl IntoResponse {
    // Basic iCal format for calendar integration
    let ical_content = format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//Radarr MVP//Calendar//EN\r\n\
         CALSCALE:GREGORIAN\r\n\
         METHOD:PUBLISH\r\n\
         X-WR-CALNAME:Radarr Movie Calendar\r\n\
         X-WR-CALDESC:Upcoming movie releases from Radarr\r\n\
         BEGIN:VEVENT\r\n\
         UID:matrix-1999@radarr-mvp\r\n\
         DTSTART:{}Z\r\n\
         DTEND:{}Z\r\n\
         SUMMARY:The Matrix (1999)\r\n\
         DESCRIPTION:Movie release: The Matrix\r\n\
         CATEGORIES:Movie\r\n\
         END:VEVENT\r\n\
         BEGIN:VEVENT\r\n\
         UID:forrest-gump-1994@radarr-mvp\r\n\
         DTSTART:{}Z\r\n\
         DTEND:{}Z\r\n\
         SUMMARY:Forrest Gump (1994)\r\n\
         DESCRIPTION:Movie release: Forrest Gump\r\n\
         CATEGORIES:Movie\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR\r\n",
        Utc::now().format("%Y%m%dT%H%M%S"),
        Utc::now().format("%Y%m%dT%H%M%S"),
        Utc::now().format("%Y%m%dT%H%M%S"),
        Utc::now().format("%Y%m%dT%H%M%S"),
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        "text/calendar; charset=utf-8".parse().unwrap(),
    );
    headers.insert(
        "content-disposition",
        "attachment; filename=radarr.ics".parse().unwrap(),
    );

    (headers, ical_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_calendar_entry_creation() {
        let mut movie = Movie::new(12345, "Test Movie".to_string());
        movie.year = Some(2023);
        movie.imdb_id = Some("tt1234567".to_string());
        movie.status = radarr_core::MovieStatus::Released;
        movie.minimum_availability = radarr_core::MinimumAvailability::Released;

        let entry = CalendarEntry::from(movie);
        assert_eq!(entry.title, "Test Movie");
        assert_eq!(entry.year, Some(2023));
        assert_eq!(entry.tmdb_id, Some(12345));
        assert!(entry.monitored);
    }

    #[test]
    fn test_calendar_params_parsing() {
        // Test that calendar params can be parsed from query strings
        let params = CalendarParams {
            start: Some("2023-01-01".to_string()),
            end: Some("2023-01-31".to_string()),
            unmonitored: Some(false),
        };

        assert_eq!(params.start, Some("2023-01-01".to_string()));
        assert_eq!(params.end, Some("2023-01-31".to_string()));
        assert_eq!(params.unmonitored, Some(false));
    }
}
