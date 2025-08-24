use radarr_core::{Movie, MovieStatus, RadarrError, circuit_breaker::{CircuitBreaker, CircuitBreakerConfig}};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
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
pub struct TmdbClient {
    client: Client,
    api_key: String,
    base_url: String,
    circuit_breaker: CircuitBreaker,
}

impl TmdbClient {
    pub fn new(api_key: String) -> Self {
        let circuit_breaker_config = CircuitBreakerConfig::new("TMDB")
            .with_failure_threshold(5)
            .with_timeout(Duration::from_secs(30))
            .with_request_timeout(Duration::from_secs(10))
            .with_success_threshold(2);
            
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.themoviedb.org/3".to_string(),
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        }
    }
    
    pub fn new_with_circuit_breaker(api_key: String, circuit_breaker_config: CircuitBreakerConfig) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.themoviedb.org/3".to_string(),
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        }
    }

    /// Search for movies by query
    pub async fn search_movies(&self, query: &str, page: Option<i32>) -> Result<Vec<Movie>, TmdbError> {
        let page = page.unwrap_or(1);
        let query_clone = query.to_string();
        let api_key_clone = self.api_key.clone();
        let base_url_clone = self.base_url.clone();
        let client_clone = self.client.clone();
        
        let result = self.circuit_breaker.call(async move {
            let url = format!("{}/search/movie", base_url_clone);
            
            debug!("Searching TMDB for movies: query={}, page={}", query_clone, page);
            
            let response = client_clone
                .get(&url)
                .query(&[
                    ("api_key", &api_key_clone),
                    ("query", &query_clone),
                    ("page", &page.to_string()),
                ])
                .send()
                .await
                .map_err(TmdbError::HttpError)?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                error!("TMDB API error: {} - {}", status, text);
                return Err(TmdbError::ApiError {
                    message: format!("HTTP {}: {}", status, text),
                });
            }

            let search_response: TmdbSearchResponse = response.json().await
                .map_err(TmdbError::HttpError)?;
            
            debug!("TMDB search returned {} results", search_response.results.len());
            
            Ok(search_response)
        }).await;
        
        match result {
            Ok(search_response) => {
                // Convert TMDB results to our Movie model
                let movies = search_response.results
                    .into_iter()
                    .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
                    .collect();
                Ok(movies)
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                Err(TmdbError::ApiError {
                    message: format!("TMDB service unavailable: circuit breaker open for {}", service),
                })
            }
            Err(RadarrError::Timeout { operation }) => {
                Err(TmdbError::ApiError {
                    message: format!("TMDB request timed out: {}", operation),
                })
            }
            Err(e) => Err(TmdbError::ApiError {
                message: format!("TMDB service error: {}", e),
            }),
        }
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

    /// Get now playing movies
    pub async fn get_now_playing(&self, page: Option<i32>) -> Result<Vec<Movie>, TmdbError> {
        let page = page.unwrap_or(1);
        let url = format!("{}/movie/now_playing", self.base_url);
        
        debug!("Fetching TMDB now playing movies: page={}", page);
        
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
        
        debug!("TMDB now playing returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get top rated movies
    pub async fn get_top_rated(&self, page: Option<i32>) -> Result<Vec<Movie>, TmdbError> {
        let page = page.unwrap_or(1);
        let url = format!("{}/movie/top_rated", self.base_url);
        
        debug!("Fetching TMDB top rated movies: page={}", page);
        
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
        
        debug!("TMDB top rated returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get movies from a collection
    pub async fn get_collection(&self, collection_id: i32) -> Result<Vec<Movie>, TmdbError> {
        let url = format!("{}/collection/{}", self.base_url, collection_id);
        
        debug!("Fetching TMDB collection: id={}", collection_id);
        
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

        let collection_response: TmdbCollectionResponse = response.json().await?;
        
        debug!("TMDB collection returned {} movies", collection_response.parts.len());
        
        // Convert TMDB results to our Movie model
        let movies = collection_response.parts
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get movies by person (actor/director)
    pub async fn get_person_movies(&self, person_id: i32) -> Result<Vec<Movie>, TmdbError> {
        let url = format!("{}/person/{}/movie_credits", self.base_url, person_id);
        
        debug!("Fetching movies for person: id={}", person_id);
        
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

        let credits_response: TmdbPersonCreditsResponse = response.json().await?;
        
        debug!("TMDB person credits returned {} movies", credits_response.cast.len());
        
        // Convert TMDB results to our Movie model (combine cast and crew)
        let mut movies = credits_response.cast
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect::<Vec<_>>();
            
        let crew_movies: Vec<Movie> = credits_response.crew
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();
            
        movies.extend(crew_movies);
        
        // Remove duplicates by TMDB ID
        movies.sort_by_key(|m| m.tmdb_id);
        movies.dedup_by_key(|m| m.tmdb_id);

        Ok(movies)
    }

    /// Get movies by keyword
    pub async fn get_keyword_movies(&self, keyword_id: i32) -> Result<Vec<Movie>, TmdbError> {
        let page = 1;
        let url = format!("{}/keyword/{}/movies", self.base_url, keyword_id);
        
        debug!("Fetching movies for keyword: id={}", keyword_id);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("page", &page.to_string()),
            ])
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

        let search_response: TmdbSearchResponse = response.json().await?;
        
        debug!("TMDB keyword movies returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get public list
    pub async fn get_list(&self, list_id: &str) -> Result<Vec<Movie>, TmdbError> {
        let url = format!("{}/list/{}", self.base_url, list_id);
        
        debug!("Fetching TMDB list: id={}", list_id);
        
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

        let list_response: TmdbListResponse = response.json().await?;
        
        debug!("TMDB list returned {} items", list_response.items.len());
        
        // Convert TMDB results to our Movie model
        let movies = list_response.items
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

    /// Get movies from a production company
    pub async fn get_company_movies(&self, company_id: i32) -> Result<Vec<Movie>, TmdbError> {
        let page = 1;
        let url = format!("{}/company/{}/movies", self.base_url, company_id);
        
        debug!("Fetching movies for company: id={}", company_id);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("page", &page.to_string()),
            ])
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

        let search_response: TmdbSearchResponse = response.json().await?;
        
        debug!("TMDB company movies returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get movies using TMDb discover endpoint with filters
    pub async fn get_discover_movies(&self, params: &[(&str, &str)]) -> Result<Vec<Movie>, TmdbError> {
        let url = format!("{}/discover/movie", self.base_url);
        
        debug!("Fetching movies via discover with {} filters", params.len());
        
        let mut query_params = vec![("api_key", self.api_key.as_str())];
        query_params.extend_from_slice(params);
        
        let response = self.client
            .get(&url)
            .query(&query_params)
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
        
        debug!("TMDB discover returned {} results", search_response.results.len());
        
        // Convert TMDB results to our Movie model
        let movies = search_response.results
            .into_iter()
            .map(|tmdb_movie| self.tmdb_movie_to_movie(tmdb_movie))
            .collect();

        Ok(movies)
    }

    /// Get circuit breaker metrics for monitoring
    pub async fn get_circuit_breaker_metrics(&self) -> radarr_core::CircuitBreakerMetrics {
        self.circuit_breaker.get_metrics().await
    }

    /// Check if TMDB service is healthy
    pub async fn is_healthy(&self) -> bool {
        self.circuit_breaker.is_healthy().await
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

/// TMDB collection response
#[derive(Debug, Deserialize)]
struct TmdbCollectionResponse {
    id: i32,
    name: String,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    parts: Vec<TmdbMovie>,
}

/// TMDB person credits response
#[derive(Debug, Deserialize)]
struct TmdbPersonCreditsResponse {
    id: i32,
    cast: Vec<TmdbMovie>,
    crew: Vec<TmdbMovie>,
}

/// TMDB list response
#[derive(Debug, Deserialize)]
struct TmdbListResponse {
    created_by: String,
    description: String,
    favorite_count: i32,
    id: String,
    items: Vec<TmdbMovie>,
    item_count: i32,
    iso_639_1: String,
    name: String,
    poster_path: Option<String>,
}