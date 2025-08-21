//! Database connection and configuration
//!
//! This module handles PostgreSQL connection setup and pool management.

use crate::error::InfrastructureError;
use radarr_core::Result;
use sqlx::{Pool, Postgres};
use std::time::Duration;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://radarr:radarr@localhost:5432/radarr".to_string(),
            max_connections: 5,  // FIXED: Was 30, causing pool exhaustion
            min_connections: 1,  // FIXED: Was 8, too many idle connections
            acquire_timeout: Duration::from_secs(3),  // FIXED: Was 30, too long
            idle_timeout: Duration::from_secs(10),    // FIXED: Was 300, connections lingering
            max_lifetime: Duration::from_secs(300),   // FIXED: Was 1800, too long
        }
    }
}

impl DatabaseConfig {
    /// Create a new database configuration from environment
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://radarr:radarr@localhost:5432/radarr".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "8".to_string())
                .parse()
                .unwrap_or(8),
            acquire_timeout: Duration::from_secs(
                std::env::var("DATABASE_ACQUIRE_TIMEOUT_SECS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            ),
            idle_timeout: Duration::from_secs(
                std::env::var("DATABASE_IDLE_TIMEOUT_SECS")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
            ),
            max_lifetime: Duration::from_secs(
                std::env::var("DATABASE_MAX_LIFETIME_SECS")
                    .unwrap_or_else(|_| "1800".to_string())
                    .parse()
                    .unwrap_or(1800),
            ),
        }
    }
}

/// Database pool wrapper
pub type DatabasePool = Pool<Postgres>;

/// Create a PostgreSQL connection pool
pub async fn create_pool(config: DatabaseConfig) -> Result<DatabasePool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .test_before_acquire(true)
        // TODO: Add session optimization after fixing connection lifetime issues
        .connect(&config.database_url)
        .await?;

    Ok(pool)
}

/// Run database migrations
pub async fn migrate(pool: &DatabasePool) -> Result<()> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| InfrastructureError::Migration(e.to_string()))?;
    
    Ok(())
}

/// Test database connection
pub async fn test_connection(pool: &DatabasePool) -> Result<()> {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await?;
    
    Ok(())
}