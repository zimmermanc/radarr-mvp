use radarr_core::{Movie, MovieStatus, RadarrError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

/// TMDB API error types
#[derive(Debug, thiserror::Error)]
pub enum TmdbError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Failed to parse TMDB response: {0}")]
    ParseError(#[from] serde_json::Error),
    
    #[error("TMDB API error: {message}")]
    ApiError { message: String },
    
    #[error("Movie not found")]
    NotFound,
}

impl From<TmdbError> for RadarrError {
    fn from(error: TmdbError) -> Self {
        RadarrError::ExternalServiceError {
            service: "tmdb".to_string(),
            error: error.to_string(),
        }
    }
}

/// TMDB API client
#[derive(Clone)]
pub struct TmdbClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl TmdbClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.themoviedb.org/3".to_string(),
        }
    }

    /// Search for movies by query
    pub async fn search_movies(&self, query: &str, page: Option<i32>) -> Result<Vec<Movie>, TmdbError> {
        let page = page.unwrap_or(1);
        let url = format!("{}/search/movie", self.base_url);
        
        debug!("Searching TMDB for movies: query={}, page={}", query, page);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("query", &query.to_string()),
                ("page", &page.to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(TmdbError::ApiError {
                message: format!("HTTP {}: {}", status, text),
            });
        }

        let search_response: TmdbSearchResponse = response.json().await?;
        
        debug!("TMDB search returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get a specific movie by TMDB ID
    pub async fn get_movie(&self, tmdb_id: i32) -> Result<Movie, TmdbError> {
        let url = format!("{}/movie/{}", self.base_url, tmdb_id);
        
        debug!("Fetching TMDB movie: id={}", tmdb_id);
        
        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.api_key)])
            .send()
            .await?;

        if response.status() == 404 {
            return Err(TmdbError::NotFound);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(TmdbError::ApiError {
                message: format!("HTTP {}: {}", status, text),
            });
        }

        let tmdb_movie: TmdbMovie = response.json().await?;
        
        debug!("TMDB movie fetched: {}", tmdb_movie.title);
        
        Ok(self.tmdb_movie_to_movie(tmdb_movie))
    }

    /// Get popular movies
    pub async fn get_popular(&self, page: Option<i32>) -> Result<Vec<Movie>, TmdbError> {
        let page = page.unwrap_or(1);
        let url = format!("{}/movie/popular", self.base_url);
        
        debug!("Fetching TMDB popular movies: page={}", page);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("page", &page.to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(TmdbError::ApiError {
                message: format!("HTTP {}: {}", status, text),
            });
        }

        let search_response: TmdbSearchResponse = response.json().await?;
        
        debug!("TMDB popular returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get upcoming movies
    pub async fn get_upcoming(&self, page: Option<i32>) -> Result<Vec<Movie>, TmdbError> {
        let page = page.unwrap_or(1);
        let url = format!("{}/movie/upcoming", self.base_url);
        
        debug!("Fetching TMDB upcoming movies: page={}", page);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("page", &page.to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(TmdbError::ApiError {
                message: format!("HTTP {}: {}", status, text),
            });
        }

        let search_response: TmdbSearchResponse = response.json().await?;
        
        debug!("TMDB upcoming returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Convert TMDB movie to our Movie model
    fn tmdb_movie_to_movie(&self, tmdb_movie: TmdbMovie) -> Movie {
        let mut movie = Movie::new(tmdb_movie.id, tmdb_movie.title.clone());
        
        // Set basic fields
        movie.original_title = Some(tmdb_movie.original_title.clone());
        movie.year = tmdb_movie.release_date
            .as_ref()
            .and_then(|date| date.split('-').next().and_then(|year| year.parse().ok()));
        movie.runtime = tmdb_movie.runtime;
        
        // Map TMDB status to our status
        movie.status = match tmdb_movie.status.as_deref() {
            Some("Released") => MovieStatus::Released,
            Some("Post Production") => MovieStatus::PostProduction,
            Some("In Production") => MovieStatus::InProduction,
            Some("Planned") | Some("Rumored") => MovieStatus::Announced,
            Some("Canceled") => MovieStatus::Cancelled,
            _ => MovieStatus::Announced,
        };

        // Set IMDB ID if available
        movie.imdb_id = tmdb_movie.imdb_id.clone();

        // Store TMDB metadata
        movie.metadata = serde_json::json!({
            "tmdb": {
                "id": tmdb_movie.id,
                "imdb_id": tmdb_movie.imdb_id,
                "title": tmdb_movie.title,
                "original_title": tmdb_movie.original_title,
                "overview": tmdb_movie.overview,
                "release_date": tmdb_movie.release_date,
                "poster_path": tmdb_movie.poster_path,
                "backdrop_path": tmdb_movie.backdrop_path,
                "vote_average": tmdb_movie.vote_average,
                "vote_count": tmdb_movie.vote_count,
                "popularity": tmdb_movie.popularity,
                "adult": tmdb_movie.adult,
                "video": tmdb_movie.video,
                "genre_ids": tmdb_movie.genre_ids,
                "original_language": tmdb_movie.original_language,
                "status": tmdb_movie.status,
                "tagline": tmdb_movie.tagline,
                "homepage": tmdb_movie.homepage,
                "budget": tmdb_movie.budget,
                "revenue": tmdb_movie.revenue
            }
        });

        movie
    }
}

/// TMDB search response
#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    page: i32,
    results: Vec<TmdbMovie>,
    total_pages: i32,
    total_results: i32,
}

/// TMDB movie data structure
#[derive(Debug, Deserialize)]
struct TmdbMovie {
    id: i32,
    title: String,
    original_title: String,
    overview: Option<String>,
    release_date: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    vote_count: Option<i32>,
    popularity: Option<f64>,
    adult: Option<bool>,
    video: Option<bool>,
    genre_ids: Option<Vec<i32>>,
    original_language: Option<String>,
    
    // Fields available in detailed movie response
    runtime: Option<i32>,
    status: Option<String>,
    tagline: Option<String>,
    homepage: Option<String>,
    budget: Option<i64>,
    revenue: Option<i64>,
    imdb_id: Option<String>,
}