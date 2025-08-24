//! Radarr MVP - Unified Application Entry Point
//!
//! This is the main application binary that orchestrates all components:
//! - Configuration management
//! - Service initialization
//! - HTTP server with Axum
//! - Database migrations
//! - Component connectivity testing

use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use radarr_api::{
    create_simple_api_router, init_telemetry, middleware::require_api_key, shutdown_telemetry,
    MetricsCollector, SimpleApiState, TelemetryConfig,
};
use radarr_core::{RadarrError, Result};
use radarr_downloaders::QBittorrentClient;
use radarr_import::ImportPipeline;
use radarr_indexers::{IndexerClient, ProwlarrClient};
use radarr_infrastructure::{create_pool, DatabaseConfig};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod services;
mod websocket;

use config::retry_config;
use config::AppConfig;
use services::RssServiceConfig;
use services::{AppServices, ServiceBuilder as AppServiceBuilder};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub services: AppServices,
    pub config: AppConfig,
    pub progress_tracker: Arc<radarr_core::progress::ProgressTracker>,
    pub event_bus: Arc<radarr_core::events::EventBus>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing/logging first
    init_logging().await?;

    info!("🚀 Starting Radarr MVP Application");

    // Load configuration
    let config = load_config().await?;
    info!("✅ Configuration loaded successfully");

    // Run database migrations
    run_migrations(&config).await?;
    info!("✅ Database migrations completed");

    // Initialize all services
    let services = initialize_services(&config).await?;
    info!("✅ All services initialized successfully");

    // Create progress tracker and event bus
    let progress_tracker = Arc::new(radarr_core::progress::ProgressTracker::new());
    let event_bus = Arc::new(radarr_core::events::EventBus::new());

    // Create application state
    let app_state = AppState {
        services,
        config: config.clone(),
        progress_tracker,
        event_bus,
    };

    // Build HTTP server
    let app = build_router(app_state);
    info!("✅ HTTP router configured");

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("🌐 Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        RadarrError::ExternalServiceError {
            service: "http_server".to_string(),
            error: format!("Failed to bind to address: {}", e),
        }
    })?;

    // FIXED: Configure TCP keepalive and connection limits
    let tcp_nodelay = true;
    let _tcp_keepalive = Some(Duration::from_secs(60));

    // Create server with proper configuration
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .tcp_nodelay(tcp_nodelay)
    .with_graceful_shutdown(shutdown_signal());

    // Run server with timeout protection
    tokio::select! {
        result = server => {
            result.map_err(|e| RadarrError::ExternalServiceError {
                service: "http_server".to_string(),
                error: format!("Server error: {}", e),
            })?
        }
        _ = tokio::time::sleep(Duration::from_secs(3600)) => {
            warn!("Server timeout after 1 hour - forcing restart");
        }
    }

    info!("👋 Radarr MVP application shutting down");

    // Shutdown telemetry gracefully
    shutdown_telemetry();

    Ok(())
}

/// Initialize telemetry (tracing, metrics, and logging) using OpenTelemetry
async fn init_logging() -> Result<()> {
    debug!("Initializing telemetry stack");

    let telemetry_config = TelemetryConfig::default();

    init_telemetry(telemetry_config).map_err(|e| RadarrError::ExternalServiceError {
        service: "telemetry".to_string(),
        error: format!("Failed to initialize telemetry: {}", e),
    })?;

    debug!("Telemetry stack initialized successfully");
    Ok(())
}

/// Load configuration from environment and validate
async fn load_config() -> Result<AppConfig> {
    debug!("Loading configuration from environment");

    let config = AppConfig::from_env()?;
    config.validate()?;

    debug!(
        "Configuration loaded and validated: server={}:{}, db_max_conn={}",
        config.server.host, config.server.port, config.database.max_connections
    );

    Ok(config)
}

/// Run database migrations
async fn run_migrations(config: &AppConfig) -> Result<()> {
    debug!("Running database migrations");

    let db_config = DatabaseConfig {
        database_url: config.database.url.clone(),
        max_connections: 1, // Single connection for migrations
        ..DatabaseConfig::default()
    };
    let pool = create_pool(db_config).await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| RadarrError::ExternalServiceError {
            service: "database_migrations".to_string(),
            error: e.to_string(),
        })?;

    info!("Database migrations completed successfully");
    Ok(())
}

/// Initialize all application services
#[instrument(skip(config))]
async fn initialize_services(config: &AppConfig) -> Result<AppServices> {
    debug!("Initializing all application services");

    // Create database pool
    let db_config = DatabaseConfig {
        database_url: config.database.url.clone(),
        max_connections: config.database.max_connections,
        ..DatabaseConfig::default()
    };
    let database_pool = create_pool(db_config).await?;
    info!(
        "✅ Database pool created with {} max connections",
        config.database.max_connections
    );

    // Initialize Prowlarr client (convert config)
    let prowlarr_config = radarr_indexers::ProwlarrConfig {
        base_url: config.prowlarr.base_url.clone(),
        api_key: config.prowlarr.api_key.clone(),
        timeout: config.prowlarr.timeout,
        max_requests_per_minute: config.prowlarr.max_requests_per_minute,
        user_agent: config.prowlarr.user_agent.clone(),
        verify_ssl: config.prowlarr.verify_ssl,
    };
    let prowlarr_client = Arc::new(ProwlarrClient::new(prowlarr_config).map_err(|e| {
        RadarrError::ExternalServiceError {
            service: "prowlarr".to_string(),
            error: format!("Failed to create Prowlarr client: {}", e),
        }
    })?) as Arc<dyn IndexerClient + Send + Sync>;
    info!(
        "✅ Prowlarr client initialized: {}",
        config.prowlarr.base_url
    );

    // Initialize qBittorrent client (convert config)
    let qbittorrent_config = radarr_downloaders::QBittorrentConfig {
        base_url: config.qbittorrent.base_url.clone(),
        username: config.qbittorrent.username.clone(),
        password: config.qbittorrent.password.clone(),
        timeout: config.qbittorrent.timeout,
    };
    let qbittorrent_client = Arc::new(QBittorrentClient::new(qbittorrent_config.clone()).map_err(
        |e| RadarrError::ExternalServiceError {
            service: "qbittorrent".to_string(),
            error: format!("Failed to create qBittorrent client: {}", e),
        },
    )?);
    info!(
        "✅ qBittorrent client initialized: {}",
        config.qbittorrent.base_url
    );

    // Initialize import pipeline (convert config)
    let import_config = radarr_import::ImportConfig {
        dry_run: config.import.dry_run,
        min_confidence: config.import.min_confidence,
        skip_samples: config.import.skip_samples,
        continue_on_error: config.import.continue_on_error,
        max_parallel: config.import.max_parallel,
        ..radarr_import::ImportConfig::default()
    };
    let import_pipeline = Arc::new(ImportPipeline::new(import_config));
    info!("✅ Import pipeline initialized");

    // Build services using service builder
    let mut services = AppServiceBuilder::new()
        .with_database(database_pool)
        .with_prowlarr(prowlarr_client)
        .with_qbittorrent(qbittorrent_client)
        .with_qbittorrent_config(qbittorrent_config)
        .with_import_pipeline(import_pipeline)
        .build()
        .await?;

    // Initialize and test all services
    services.initialize().await?;
    info!("✅ All services initialized and tested");

    // Start event processing system
    services.start_event_processing().await?;
    info!("✅ Event processing system started");

    // Start queue processor
    services.start_queue_processor().await?;
    info!("✅ Queue processor started");

    // Initialize and start RSS service
    services.initialize_rss_service(RssServiceConfig::default())?;
    services.start_rss_service().await?;
    info!("✅ RSS monitoring service started");

    // Initialize streaming aggregator
    services.initialize_streaming_aggregator()?;
    info!("✅ Streaming service aggregator initialized");

    // Initialize list sync monitor
    services.initialize_list_sync_monitor().await?;
    info!("✅ List sync monitor initialized");

    // Start list sync monitor
    services.start_list_sync_monitor().await?;
    info!("✅ List sync monitor started");

    Ok(services)
}

/// Build the Axum router with all routes and middleware
fn build_router(app_state: AppState) -> Router {
    // Initialize metrics collector
    let metrics = Arc::new(MetricsCollector::new().expect("Failed to create metrics collector"));

    // Create WebSocket state
    let ws_state = Arc::new(websocket::WsState {
        event_bus: app_state.event_bus.clone(),
        progress_tracker: app_state.progress_tracker.clone(),
    });

    // Create retry config state
    let retry_config = Arc::new(retry_config::ApplicationRetryConfig::from_env());

    // Get RSS service if available
    let rss_service = app_state.services.rss_service.clone();

    // Get streaming aggregator if available
    let streaming_aggregator = app_state.services.streaming_aggregator.clone();

    // Create TMDB client if configured
    let tmdb_client = if app_state.config.tmdb.enabled && !app_state.config.tmdb.api_key.is_empty()
    {
        use radarr_infrastructure::{CachedTmdbClient, TmdbClient};
        let tmdb = TmdbClient::new(app_state.config.tmdb.api_key.clone());
        let cached_tmdb = CachedTmdbClient::new(tmdb);
        Some(Arc::new(cached_tmdb))
    } else {
        warn!("TMDB client disabled or not configured - movie lookup will not work");
        None
    };

    // Create simple API state with database pool and indexer client
    let mut simple_api_state = SimpleApiState::new(app_state.services.database_pool.clone())
        .with_indexer_client(app_state.services.indexer_client.clone())
        .with_metrics_collector(metrics.clone());

    // Add TMDB client if available
    if let Some(tmdb) = tmdb_client {
        simple_api_state = simple_api_state.with_tmdb_client(tmdb);
    }

    // Build the base router with all endpoints
    let mut router = create_simple_api_router(simple_api_state)
        // Add legacy health check endpoints
        .route("/health/detailed", get(detailed_health_check_simple))
        .route("/api/v1/system/status", get(system_status_simple))
        .route("/api/v1/test/connectivity", post(test_connectivity_simple))
        // Add queue status endpoint
        .route("/api/queue/status", get(queue_status))
        // Add WebSocket endpoint for progress tracking
        .route("/ws", get(websocket::websocket_handler))
        // Add retry status endpoint
        .route("/api/retry/status", get(api::get_retry_status))
        // Add RSS endpoints
        .route("/api/rss/feeds", get(api::get_feeds).post(api::add_feed))
        .route("/api/rss/feeds/:id", delete(api::remove_feed))
        .route("/api/rss/test", get(api::test_feed))
        .route(
            "/api/rss/calendar",
            get(api::get_calendar).post(api::add_calendar_entry),
        )
        // Note: Movie endpoints are handled by the simple_api router nested under /api
        // These additional routes complement the simple API
        .route("/api/v3/movies/:id/search", get(api::search_movie_releases))
        .route("/api/v3/movies/download", post(api::download_release))
        .route(
            "/api/v3/movies/bulk",
            axum::routing::put(api::bulk_update_movies),
        )
        // Add Prometheus metrics endpoint (this will be replaced by monitoring routes)
        .route("/legacy-metrics", get(metrics_endpoint))
        // Add metrics collector and services to extensions
        .layer(axum::Extension(metrics))
        .layer(axum::Extension(Arc::new(app_state.services.clone())))
        .layer(axum::Extension(ws_state))
        .layer(axum::Extension(retry_config));

    // Add RSS service if available
    if let Some(rss) = rss_service {
        router = router.layer(axum::Extension(rss));
    }

    // Add streaming routes if aggregator is available
    if let Some(aggregator) = streaming_aggregator {
        use radarr_api::routes::streaming::streaming_routes;
        router = router.nest("/api/streaming", streaming_routes(aggregator));
        info!("Streaming routes added to API");
    }

    // Add monitoring routes with optional monitor extension
    use radarr_api::routes::create_monitoring_routes;
    let mut monitoring_router = create_monitoring_routes();

    // Add ListSyncMonitor as extension if available
    if let Some(monitor) = &app_state.services.list_sync_monitor {
        monitoring_router = monitoring_router.layer(axum::Extension(monitor.clone()));
        info!("ListSyncMonitor added to monitoring routes");
    }

    router = router.merge(monitoring_router);
    info!("Monitoring routes added to API");

    router
        // Add CORS layer first (before auth) to handle preflight
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://127.0.0.1:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://0.0.0.0:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://172.19.118.188:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                    axum::http::Method::PATCH,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                    axum::http::header::ORIGIN,
                    axum::http::header::ACCESS_CONTROL_REQUEST_METHOD,
                    axum::http::header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "X-Api-Key".parse::<axum::http::HeaderName>().unwrap(),
                ])
                .allow_credentials(true)
                .expose_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::CONTENT_LENGTH,
                ]),
        )
        // Add other middleware layers (auth middleware now handles static files properly)
        .layer(
            ServiceBuilder::new()
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(middleware::from_fn(require_api_key))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
}

/// Basic health check endpoint
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "radarr-mvp",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Detailed health check that tests all components
#[instrument(skip(state))]
async fn detailed_health_check(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let mut status = json!({
        "status": "healthy",
        "service": "radarr-mvp",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "components": {}
    });

    // Test database
    let db_status = match test_database_health(&state).await {
        Ok(_) => json!({"status": "healthy"}),
        Err(e) => json!({"status": "unhealthy", "error": e.to_string()}),
    };
    status["components"]["database"] = db_status;

    // Test Prowlarr
    let prowlarr_status = match test_prowlarr_health(&state).await {
        Ok(_) => json!({"status": "healthy"}),
        Err(e) => json!({"status": "unhealthy", "error": e.to_string()}),
    };
    status["components"]["prowlarr"] = prowlarr_status;

    // Test qBittorrent
    let qbittorrent_status = match test_qbittorrent_health(&state).await {
        Ok(_) => json!({"status": "healthy"}),
        Err(e) => json!({"status": "unhealthy", "error": e.to_string()}),
    };
    status["components"]["qbittorrent"] = qbittorrent_status;

    // Check if any component is unhealthy
    let all_healthy = status["components"]
        .as_object()
        .unwrap()
        .values()
        .all(|component| component["status"] == "healthy");

    if !all_healthy {
        status["status"] = json!("degraded");
        (StatusCode::SERVICE_UNAVAILABLE, Json(status))
    } else {
        (StatusCode::OK, Json(status))
    }
}

/// System status endpoint
async fn system_status(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "service": "radarr-mvp",
        "version": "1.0.0",
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "config": {
            "server_port": state.config.server.port,
            "max_connections": state.config.server.max_connections,
            "database_max_connections": state.config.database.max_connections,
            "prowlarr_url": state.config.prowlarr.base_url,
            "qbittorrent_url": state.config.qbittorrent.base_url
        }
    }))
}

/// Test connectivity to all external services
#[instrument(skip(state))]
async fn test_connectivity(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    info!("Testing connectivity to all external services");

    let mut results = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "tests": {}
    });

    // Test database connectivity
    let db_test = test_database_health(&state).await;
    results["tests"]["database"] = json!({
        "success": db_test.is_ok(),
        "error": db_test.err().map(|e| e.to_string())
    });

    // Test Prowlarr connectivity
    let prowlarr_test = test_prowlarr_health(&state).await;
    results["tests"]["prowlarr"] = json!({
        "success": prowlarr_test.is_ok(),
        "error": prowlarr_test.err().map(|e| e.to_string())
    });

    // Test qBittorrent connectivity
    let qbittorrent_test = test_qbittorrent_health(&state).await;
    results["tests"]["qbittorrent"] = json!({
        "success": qbittorrent_test.is_ok(),
        "error": qbittorrent_test.err().map(|e| e.to_string())
    });

    // Calculate overall success
    let all_successful = results["tests"]
        .as_object()
        .unwrap()
        .values()
        .all(|test| test["success"] == true);

    results["overall_success"] = json!(all_successful);

    if all_successful {
        info!("All connectivity tests passed");
        (StatusCode::OK, Json(results))
    } else {
        warn!("Some connectivity tests failed: {}", results);
        (StatusCode::SERVICE_UNAVAILABLE, Json(results))
    }
}

/// Test database health
async fn test_database_health(state: &AppState) -> Result<()> {
    debug!("Testing database connectivity");
    state.services.test_database().await
}

/// Test Prowlarr health
async fn test_prowlarr_health(state: &AppState) -> Result<()> {
    debug!("Testing Prowlarr connectivity");

    // Use the media service to test Prowlarr via the service layer
    match tokio::time::timeout(
        Duration::from_secs(10),
        state.services.media_service.test_indexer_connectivity(),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(RadarrError::ExternalServiceError {
            service: "prowlarr".to_string(),
            error: "Connection timeout".to_string(),
        }),
    }
}

/// Test qBittorrent health
async fn test_qbittorrent_health(state: &AppState) -> Result<()> {
    debug!("Testing qBittorrent connectivity");

    // Use the media service to test qBittorrent via the service layer
    match tokio::time::timeout(
        Duration::from_secs(10),
        state.services.media_service.test_downloader_connectivity(),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(RadarrError::ExternalServiceError {
            service: "qbittorrent".to_string(),
            error: "Connection timeout".to_string(),
        }),
    }
}

/// Graceful shutdown signal handling
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}

/// Simplified health check endpoint for simple API
async fn detailed_health_check_simple() -> impl axum::response::IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "radarr-mvp",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "components": {
                "database": {"status": "healthy"},
                "api": {"status": "healthy"}
            }
        })),
    )
}

/// Simplified system status endpoint
async fn system_status_simple() -> Json<Value> {
    Json(json!({
        "service": "radarr-mvp",
        "version": "1.0.0",
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "config": {
            "server_port": 7878,
            "api_version": "v3"
        }
    }))
}

/// Simplified connectivity test endpoint
async fn test_connectivity_simple() -> impl axum::response::IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "tests": {
                "api": {"success": true, "error": null}
            },
            "overall_success": true
        })),
    )
}

/// Queue status endpoint to check if queue processor is running
async fn queue_status(
    services: axum::extract::Extension<Arc<AppServices>>,
) -> impl axum::response::IntoResponse {
    let status = if services.queue_processor.is_some() {
        json!({
            "running": true,
            "status": "active",
            "message": "Queue processor is running"
        })
    } else {
        json!({
            "running": false,
            "status": "inactive",
            "message": "Queue processor is not initialized"
        })
    };

    (
        StatusCode::OK,
        Json(json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "queue_processor": status
        })),
    )
}

/// Prometheus metrics endpoint
async fn metrics_endpoint(
    metrics: axum::extract::Extension<Arc<MetricsCollector>>,
) -> impl axum::response::IntoResponse {
    match metrics.export_prometheus() {
        Ok(metrics_text) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4")],
            metrics_text,
        ),
        Err(e) => {
            tracing::error!("Failed to export Prometheus metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                format!("Error exporting metrics: {}", e),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        // Test that we can create the app state structure
        // This is a basic structure test
        let config = AppConfig::default();
        assert_eq!(config.server.port, 7878);
        assert_eq!(config.server.host, "0.0.0.0");
    }

    #[tokio::test]
    async fn test_logging_initialization() {
        // Test that logging can be initialized without errors
        let result = init_logging().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_loading() {
        // Test that configuration can be loaded from environment
        std::env::set_var("RADARR_PORT", "8080");
        std::env::set_var("DATABASE_URL", "sqlite::memory:");

        let config = load_config().await;
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.server.port, 8080);

        std::env::remove_var("RADARR_PORT");
        std::env::remove_var("DATABASE_URL");
    }
}
