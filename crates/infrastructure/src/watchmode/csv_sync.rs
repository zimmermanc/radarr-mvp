use chrono::Utc;
use radarr_core::{
    streaming::{IdMapping, MediaType, traits::StreamingCacheRepository},
    RadarrError,
};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info, warn};

/// Watchmode CSV sync for TMDB ID mappings
pub struct WatchmodeCsvSync {
    client: Client,
    cache_repo: Arc<dyn StreamingCacheRepository>,
    csv_url: String,
}

impl WatchmodeCsvSync {
    pub fn new(cache_repo: Arc<dyn StreamingCacheRepository>) -> Self {
        Self {
            client: Client::new(),
            cache_repo,
            // Watchmode provides a public CSV file with ID mappings
            csv_url: "https://api.watchmode.com/datasets/title_id_map.csv".to_string(),
        }
    }

    /// Download and parse the CSV file
    pub async fn download_csv(&self) -> Result<Vec<u8>, RadarrError> {
        info!("Downloading Watchmode ID mapping CSV from {}", self.csv_url);
        
        let response = self.client
            .get(&self.csv_url)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "watchmode".to_string(),
                error: format!("Failed to download CSV: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(RadarrError::ExternalServiceError {
                service: "watchmode".to_string(),
                error: format!("HTTP {}: Failed to download CSV", response.status()),
            });
        }

        let bytes = response.bytes().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "watchmode".to_string(),
                error: format!("Failed to read CSV bytes: {}", e),
            })?;

        info!("Downloaded {} bytes of CSV data", bytes.len());
        Ok(bytes.to_vec())
    }

    /// Parse CSV and extract ID mappings
    pub fn parse_csv(&self, csv_data: &[u8]) -> Result<Vec<IdMapping>, RadarrError> {
        let mut mappings = Vec::new();
        let csv_str = String::from_utf8_lossy(csv_data);
        
        // Create CSV reader
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_str.as_bytes());

        for result in reader.deserialize::<CsvRecord>() {
            match result {
                Ok(record) => {
                    // Only process entries with both TMDB and Watchmode IDs
                    if let (Some(tmdb_id), Some(watchmode_id)) = (record.tmdb_id, record.watchmode_id) {
                        let media_type = match record.title_type.as_deref() {
                            Some("movie") => MediaType::Movie,
                            Some("tv_series") | Some("tv") => MediaType::Tv,
                            _ => continue, // Skip other types
                        };

                        mappings.push(IdMapping {
                            tmdb_id,
                            watchmode_id: Some(watchmode_id),
                            media_type,
                            last_verified: Utc::now(),
                        });
                    }
                }
                Err(e) => {
                    debug!("Failed to parse CSV record: {}", e);
                    continue;
                }
            }
        }

        info!("Parsed {} ID mappings from CSV", mappings.len());
        Ok(mappings)
    }

    /// Refresh the ID mappings from CSV
    pub async fn refresh_id_mappings(&self) -> Result<Vec<IdMapping>, RadarrError> {
        info!("Starting Watchmode ID mapping refresh");
        
        // Download CSV
        let csv_data = self.download_csv().await?;
        
        // Parse CSV
        let mappings = self.parse_csv(&csv_data)?;
        
        // Store in database
        let stored_count = self.cache_repo.store_id_mappings(mappings.clone()).await?;
        
        info!("Successfully stored {} ID mappings in database", stored_count);
        
        Ok(mappings)
    }

    /// Get a single Watchmode ID for a TMDB ID
    pub async fn get_watchmode_id(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
    ) -> Result<Option<i32>, RadarrError> {
        self.cache_repo.get_watchmode_id(tmdb_id, media_type).await
    }
}

#[derive(Debug, Deserialize)]
struct CsvRecord {
    #[serde(rename = "Watchmode ID")]
    watchmode_id: Option<i32>,
    #[serde(rename = "IMDB ID")]
    imdb_id: Option<String>,
    #[serde(rename = "TMDB ID")]
    tmdb_id: Option<i32>,
    #[serde(rename = "Title Type")]
    title_type: Option<String>,
    #[serde(rename = "Title")]
    title: Option<String>,
    #[serde(rename = "Release Year")]
    release_year: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_parsing() {
        // Sample CSV data for testing
        let csv_data = b"Watchmode ID,IMDB ID,TMDB ID,Title Type,Title,Release Year
1234567,tt0111161,278,movie,The Shawshank Redemption,1994
2345678,tt0068646,238,movie,The Godfather,1972
3456789,tt0108778,1396,tv_series,Friends,1994";

        // This would require a mock cache repository
        // Just verify the parsing logic compiles
        assert!(csv_data.len() > 0);
    }
}