//! Common test utilities and setup

use axum::Router;
use radarr_api::handlers::AppState;
use radarr_core::{
    models::{MinimumAvailability, Movie, MovieStatus},
    repositories::MovieRepository,
};
use radarr_infrastructure::{
    database::{create_pool, migrate, DatabaseConfig},
    repositories::PostgresMovieRepository,
    DatabasePool,
};
use uuid::Uuid;

/// Test database configuration
pub struct TestDatabase {
    pub database_name: String,
    pub pool: DatabasePool,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let test_db_name = format!("radarr_test_{}", Uuid::new_v4().simple());

        // Connect to default database to create test database
        let default_config = DatabaseConfig {
            database_url: "postgresql://radarr:radarr@localhost:5432/postgres".to_string(),
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        };

        let default_pool = create_pool(default_config)
            .await
            .expect("Failed to connect to default database");

        // Create test database
        sqlx::query(&format!("CREATE DATABASE \"{}\"", test_db_name))
            .execute(&default_pool)
            .await
            .expect("Failed to create test database");

        // Connect to test database
        let test_config = DatabaseConfig {
            database_url: format!("postgresql://radarr:radarr@localhost:5432/{}", test_db_name),
            max_connections: 10,
            min_connections: 2,
            ..Default::default()
        };

        let test_pool = create_pool(test_config)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        migrate(&test_pool)
            .await
            .expect("Failed to run migrations on test database");

        Self {
            database_name: test_db_name,
            pool: test_pool,
        }
    }

    pub async fn cleanup(self) {
        // Close all connections
        self.pool.close().await;

        // Connect to default database to drop test database
        let default_config = DatabaseConfig {
            database_url: "postgresql://radarr:radarr@localhost:5432/postgres".to_string(),
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        };

        let default_pool = create_pool(default_config)
            .await
            .expect("Failed to connect to default database for cleanup");

        // Terminate connections to test database
        sqlx::query(&format!(
            "SELECT pg_terminate_backend(pg_stat_activity.pid) FROM pg_stat_activity WHERE pg_stat_activity.datname = '{}' AND pid <> pg_backend_pid()",
            self.database_name
        ))
        .execute(&default_pool)
        .await
        .ok(); // Ignore errors here

        // Drop test database
        sqlx::query(&format!(
            "DROP DATABASE IF EXISTS \"{}\"",
            self.database_name
        ))
        .execute(&default_pool)
        .await
        .expect("Failed to drop test database");
    }
}

/// Test context with database and mocked services
pub struct TestContext {
    pub database: TestDatabase,
    pub pool: DatabasePool,
}

impl TestContext {
    pub async fn new() -> Self {
        let database = TestDatabase::new().await;
        let pool = database.pool.clone();

        Self { database, pool }
    }

    pub async fn cleanup(self) {
        self.database.cleanup().await;
    }
}

/// Create a test Axum app with mocked external services
pub async fn create_test_app(pool: DatabasePool) -> Router {
    use axum::routing::{delete, get, post, put};
    use radarr_api::handlers::{
        commands::{get_download_status, import_download, start_download, CommandState},
        health::health_check,
        movies::{create_movie, delete_movie, get_movie, list_movies, update_movie},
        search::{search_releases, SearchState},
    };

    let app_state = AppState::new(pool.clone());
    let search_state = SearchState {
        database_pool: pool.clone(),
    };
    let command_state = CommandState::new(pool.clone());

    // Create movie routes with app_state
    let movie_routes = Router::new()
        .route("/api/v3/movie", get(list_movies).post(create_movie))
        .route(
            "/api/v3/movie/:id",
            get(get_movie).put(update_movie).delete(delete_movie),
        )
        .with_state(app_state);

    // Create search routes with search_state
    let search_routes = Router::new()
        .route("/api/v3/release", get(search_releases))
        .with_state(search_state);

    // Create command routes with command_state
    let command_routes = Router::new()
        .route("/api/v3/command/download", post(start_download))
        .route("/api/v3/download/:id", get(get_download_status))
        .route("/api/v3/command/import/:id", post(import_download))
        .with_state(command_state.clone());

    // Create health routes with command_state (just for convenience)
    let health_routes = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(health_check))
        .with_state(command_state);

    // Merge all routes
    movie_routes
        .merge(search_routes)
        .merge(command_routes)
        .merge(health_routes)
}

/// Create a test movie in the database
pub async fn create_test_movie(pool: &DatabasePool) -> Movie {
    let movie_repo = PostgresMovieRepository::new(pool.clone());

    let mut movie = Movie::new(12345, "Test Movie".to_string());
    movie.year = Some(2023);
    movie.status = MovieStatus::Released;
    movie.minimum_availability = MinimumAvailability::Released;
    movie.monitored = true;
    movie.metadata = serde_json::json!({
        "tmdb": {
            "overview": "A test movie for integration tests",
            "vote_average": 7.5,
            "release_date": "2023-01-01"
        }
    });

    movie_repo
        .create(&movie)
        .await
        .expect("Failed to create test movie")
}

/// Create multiple test movies for testing pagination and lists
pub async fn create_test_movies(pool: &DatabasePool, count: usize) -> Vec<Movie> {
    let movie_repo = PostgresMovieRepository::new(pool.clone());
    let mut movies = Vec::new();

    for i in 0..count {
        let mut movie = Movie::new(10000 + i as i32, format!("Test Movie {}", i + 1));
        movie.year = Some(2020 + (i % 4) as i32);
        movie.status = MovieStatus::Released;
        movie.minimum_availability = MinimumAvailability::Released;
        movie.monitored = i % 2 == 0; // Alternate monitored status

        let created_movie = movie_repo
            .create(&movie)
            .await
            .expect("Failed to create test movie");
        movies.push(created_movie);
    }

    movies
}

/// Helper to wait for a condition with timeout
pub async fn wait_for_condition<F, Fut>(
    mut condition: F,
    timeout_secs: u64,
    check_interval_ms: u64,
) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let timeout = tokio::time::Duration::from_secs(timeout_secs);
    let interval = tokio::time::Duration::from_millis(check_interval_ms);
    let start = tokio::time::Instant::now();

    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(interval).await;
    }

    false
}

/// Create test configuration for external services
pub struct TestConfig {
    pub prowlarr_base_url: String,
    pub prowlarr_api_key: String,
    pub qbittorrent_base_url: String,
    pub qbittorrent_username: String,
    pub qbittorrent_password: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            prowlarr_base_url: "http://localhost:9696".to_string(),
            prowlarr_api_key: "test-prowlarr-key".to_string(),
            qbittorrent_base_url: "http://localhost:8080".to_string(),
            qbittorrent_username: "admin".to_string(),
            qbittorrent_password: "adminpass".to_string(),
        }
    }
}

/// Assertions for test validation
pub mod assertions {
    use radarr_api::models::*;
    use uuid::Uuid;

    pub fn assert_valid_movie_response(movie: &MovieResponse) {
        assert!(!movie.id.is_nil(), "Movie ID should not be nil");
        assert!(movie.tmdb_id > 0, "TMDB ID should be positive");
        assert!(!movie.title.is_empty(), "Movie title should not be empty");
        assert!(
            movie.created_at <= movie.updated_at,
            "Created time should be <= updated time"
        );
    }

    pub fn assert_valid_paginated_response<T>(response: &PaginatedResponse<T>) {
        assert!(response.page >= 1, "Page should be >= 1");
        assert!(response.page_size > 0, "Page size should be > 0");
        assert!(response.total_records >= 0, "Total records should be >= 0");
        assert!(response.total_pages >= 0, "Total pages should be >= 0");
        assert!(
            response.records.len() <= response.page_size as usize,
            "Records length should not exceed page size"
        );
    }

    pub fn assert_valid_release_response(release: &ReleaseResponse) {
        assert!(!release.guid.is_empty(), "Release GUID should not be empty");
        assert!(
            !release.title.is_empty(),
            "Release title should not be empty"
        );
        assert!(
            !release.download_url.is_empty(),
            "Download URL should not be empty"
        );
        assert!(release.indexer_id > 0, "Indexer ID should be positive");
        assert!(
            release.progress >= 0.0 && release.progress <= 100.0,
            "Progress should be 0-100"
        );
    }

    pub fn assert_valid_download_response(download: &DownloadResponse) {
        assert!(!download.id.is_nil(), "Download ID should not be nil");
        assert!(
            !download.status.is_empty(),
            "Download status should not be empty"
        );
        assert!(
            download.progress >= 0.0 && download.progress <= 100.0,
            "Progress should be 0-100"
        );
        if download.movie_id.is_some() {
            assert!(
                !download.movie_id.unwrap().is_nil(),
                "Movie ID should not be nil if present"
            );
        }
    }
}
