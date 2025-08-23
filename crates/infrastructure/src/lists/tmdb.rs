use super::common::{ListItem, ListParseError, ListParser, ListSource};
use crate::tmdb::TmdbClient;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// TMDb List Client for collections and lists
pub struct TmdbListClient {
    tmdb_client: TmdbClient,
}

impl TmdbListClient {
    pub fn new(api_key: String) -> Self {
        Self {
            tmdb_client: TmdbClient::new(api_key),
        }
    }
    
    /// Get popular movies
    pub async fn get_popular(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb popular movies");
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get upcoming movies
    pub async fn get_upcoming(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb upcoming movies");
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get now playing movies
    pub async fn get_now_playing(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb now playing movies");
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get top rated movies
    pub async fn get_top_rated(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb top rated movies");
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get movies from a collection (e.g., Marvel Cinematic Universe)
    pub async fn get_collection(&self, collection_id: i32) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb collection {}", collection_id);
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get movies by a specific person (actor/director)
    pub async fn get_person_movies(&self, person_id: i32) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching movies for person {}", person_id);
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get movies by keyword
    pub async fn get_keyword_movies(&self, keyword_id: i32) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching movies for keyword {}", keyword_id);
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Get a public list
    pub async fn get_list(&self, list_id: &str) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb list {}", list_id);
        // TODO: Implement using existing TMDb client
        Ok(vec![])
    }
    
    /// Convert TMDb movie to ListItem
    fn convert_movie_to_item(&self, movie: serde_json::Value) -> ListItem {
        ListItem {
            tmdb_id: movie["id"].as_i64().map(|id| id as i32),
            imdb_id: movie["imdb_id"].as_str().map(String::from),
            title: movie["title"].as_str().unwrap_or("").to_string(),
            year: movie["release_date"]
                .as_str()
                .and_then(|date| date.split('-').next())
                .and_then(|year| year.parse::<i32>().ok()),
            overview: movie["overview"].as_str().map(String::from),
            poster_path: movie["poster_path"].as_str().map(String::from),
            backdrop_path: movie["backdrop_path"].as_str().map(String::from),
            release_date: movie["release_date"].as_str().map(String::from),
            runtime: movie["runtime"].as_i64().map(|r| r as i32),
            genres: movie["genres"]
                .as_array()
                .map(|genres| {
                    genres
                        .iter()
                        .filter_map(|g| g["name"].as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            original_language: movie["original_language"].as_str().map(String::from),
            vote_average: movie["vote_average"].as_f64().map(|v| v as f32),
            vote_count: movie["vote_count"].as_i64().map(|v| v as i32),
            popularity: movie["popularity"].as_f64().map(|p| p as f32),
            source_metadata: serde_json::json!({
                "source": "tmdb",
                "tmdb_id": movie["id"],
            }),
        }
    }
}

#[async_trait]
impl ListParser for TmdbListClient {
    async fn parse_list(&self, list_url: &str) -> Result<Vec<ListItem>, ListParseError> {
        if !self.validate_url(list_url) {
            return Err(ListParseError::InvalidUrl(format!("Invalid TMDb URL: {}", list_url)));
        }
        
        // Parse the URL to determine list type
        if list_url.contains("popular") {
            self.get_popular().await
        } else if list_url.contains("upcoming") {
            self.get_upcoming().await
        } else if list_url.contains("now_playing") {
            self.get_now_playing().await
        } else if list_url.contains("top_rated") {
            self.get_top_rated().await
        } else {
            // Try to extract list ID from URL
            self.get_list(list_url).await
        }
    }
    
    fn source_type(&self) -> ListSource {
        ListSource::TMDb
    }
    
    fn validate_url(&self, url: &str) -> bool {
        // Accept special keywords or TMDb URLs
        matches!(url, "popular" | "upcoming" | "now_playing" | "top_rated") 
            || url.starts_with("https://www.themoviedb.org/")
            || url.starts_with("http://www.themoviedb.org/")
    }
}