//! Release domain model

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A release candidate from an indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: Uuid,
    pub indexer_id: i32,
    pub movie_id: Option<Uuid>,

    // Release identification
    pub title: String,
    pub info_url: Option<String>,
    pub download_url: String,
    pub guid: String,

    // Release details
    pub size_bytes: Option<i64>,
    pub age_hours: Option<i32>,
    pub seeders: Option<i32>,
    pub leechers: Option<i32>,

    // Quality information
    pub quality: serde_json::Value,

    // Protocol information
    pub protocol: ReleaseProtocol,

    // Timestamps
    pub published_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Release protocol type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseProtocol {
    Torrent,
    Usenet,
}

impl Release {
    /// Create a new release
    pub fn new(
        indexer_id: i32,
        title: String,
        download_url: String,
        guid: String,
        protocol: ReleaseProtocol,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            indexer_id,
            movie_id: None,
            title,
            info_url: None,
            download_url,
            guid,
            size_bytes: None,
            age_hours: None,
            seeders: None,
            leechers: None,
            quality: serde_json::json!({}),
            protocol,
            published_date: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Set the associated movie
    pub fn set_movie_id(&mut self, movie_id: Uuid) {
        self.movie_id = Some(movie_id);
    }

    /// Set release metrics
    pub fn set_metrics(&mut self, seeders: Option<i32>, leechers: Option<i32>) {
        self.seeders = seeders;
        self.leechers = leechers;
    }

    /// Set release size
    pub fn set_size(&mut self, size_bytes: i64) {
        self.size_bytes = Some(size_bytes);
    }

    /// Calculate release score for ranking
    pub fn calculate_score(&self) -> i32 {
        let mut score = 0;

        // Bonus for seeders (torrent only)
        if self.protocol == ReleaseProtocol::Torrent {
            if let Some(seeders) = self.seeders {
                score += seeders.min(100); // Cap at 100 points
            }
        }

        // Penalty for age (prefer newer releases)
        if let Some(age_hours) = self.age_hours {
            let age_penalty = age_hours / 24; // 1 point per day
            score -= age_penalty.min(30); // Cap penalty at 30 points
        }

        score
    }

    /// Get human readable size
    pub fn human_readable_size(&self) -> Option<String> {
        self.size_bytes.map(|bytes| {
            const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
            let mut size = bytes as f64;
            let mut unit_index = 0;

            while size >= 1024.0 && unit_index < UNITS.len() - 1 {
                size /= 1024.0;
                unit_index += 1;
            }

            format!("{:.1} {}", size, UNITS[unit_index])
        })
    }
}
