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
        
        let movies = self.tmdb_client.get_popular(Some(1))
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} popular movies to list items", items.len());
        Ok(items)
    }
    
    /// Get upcoming movies
    pub async fn get_upcoming(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb upcoming movies");
        
        let movies = self.tmdb_client.get_upcoming(Some(1))
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} upcoming movies to list items", items.len());
        Ok(items)
    }
    
    /// Get now playing movies
    pub async fn get_now_playing(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb now playing movies");
        
        let movies = self.get_now_playing_from_api(1)
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} now playing movies to list items", items.len());
        Ok(items)
    }
    
    /// Get top rated movies
    pub async fn get_top_rated(&self) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb top rated movies");
        
        let movies = self.get_top_rated_from_api(1)
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} top rated movies to list items", items.len());
        Ok(items)
    }
    
    /// Get movies from a collection (e.g., Marvel Cinematic Universe)
    pub async fn get_collection(&self, collection_id: i32) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb collection {}", collection_id);
        
        let movies = self.get_collection_from_api(collection_id)
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} collection movies to list items", items.len());
        Ok(items)
    }
    
    /// Get movies by a specific person (actor/director)
    pub async fn get_person_movies(&self, person_id: i32) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching movies for person {}", person_id);
        
        let movies = self.get_person_movies_from_api(person_id)
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} person movies to list items", items.len());
        Ok(items)
    }
    
    /// Get movies by keyword
    pub async fn get_keyword_movies(&self, keyword_id: i32) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching movies for keyword {}", keyword_id);
        
        let movies = self.get_keyword_movies_from_api(keyword_id)
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} keyword movies to list items", items.len());
        Ok(items)
    }
    
    /// Get a public list
    pub async fn get_list(&self, list_id: &str) -> Result<Vec<ListItem>, ListParseError> {
        info!("Fetching TMDb list {}", list_id);
        
        let movies = self.get_list_from_api(list_id)
            .await
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
        
        let items = movies.into_iter()
            .map(|movie| self.movie_to_list_item(movie))
            .collect();
            
        debug!("Converted {} list movies to list items", items.len());
        Ok(items)
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

    /// Convert Movie to ListItem
    fn movie_to_list_item(&self, movie: radarr_core::Movie) -> ListItem {
        // Extract TMDb metadata from the movie's metadata field
        let tmdb_metadata = movie.metadata.get("tmdb").unwrap_or(&serde_json::Value::Null);
        
        ListItem {
            tmdb_id: Some(movie.tmdb_id),
            imdb_id: movie.imdb_id.clone(),
            title: movie.title.clone(),
            year: movie.year,
            overview: tmdb_metadata["overview"].as_str().map(String::from),
            poster_path: tmdb_metadata["poster_path"].as_str().map(String::from),
            backdrop_path: tmdb_metadata["backdrop_path"].as_str().map(String::from),
            release_date: tmdb_metadata["release_date"].as_str().map(String::from),
            runtime: movie.runtime,
            genres: vec![], // Genre conversion would need additional API call
            original_language: tmdb_metadata["original_language"].as_str().map(String::from),
            vote_average: tmdb_metadata["vote_average"].as_f64().map(|v| v as f32),
            vote_count: tmdb_metadata["vote_count"].as_i64().map(|v| v as i32),
            popularity: tmdb_metadata["popularity"].as_f64().map(|p| p as f32),
            source_metadata: serde_json::json!({
                "source": "tmdb",
                "tmdb_id": movie.tmdb_id,
                "imdb_id": movie.imdb_id,
                "status": movie.status,
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