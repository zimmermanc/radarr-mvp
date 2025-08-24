use crate::tmdb::TmdbClient;
use radarr_core::models::Movie;
use std::time::Duration;
use tracing::{debug, info};

/// Simple wrapper for TMDB client (caching disabled for MVP)
pub struct CachedTmdbClient {
    client: TmdbClient,
}

impl CachedTmdbClient {
    pub fn new(client: TmdbClient) -> Self {
        Self { client }
    }

    pub async fn get_movie(&self, tmdb_id: i32) -> Result<Movie, crate::tmdb::TmdbError> {
        debug!("Fetching TMDB movie: id={}", tmdb_id);
        self.client.get_movie(tmdb_id).await
    }

    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<i32>,
    ) -> Result<Vec<Movie>, crate::tmdb::TmdbError> {
        debug!("Searching TMDB movies: query={}, page={:?}", query, page);
        self.client.search_movies(query, page).await
    }

    pub async fn get_popular(
        &self,
        page: Option<i32>,
    ) -> Result<Vec<Movie>, crate::tmdb::TmdbError> {
        debug!("Fetching TMDB popular movies: page={:?}", page);
        self.client.get_popular(page).await
    }

    pub async fn get_upcoming(
        &self,
        page: Option<i32>,
    ) -> Result<Vec<Movie>, crate::tmdb::TmdbError> {
        debug!("Fetching TMDB upcoming movies: page={:?}", page);
        self.client.get_upcoming(page).await
    }
}
