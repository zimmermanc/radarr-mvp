use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub test_on_acquire: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 32,      // Maximum connections in pool
            min_connections: 5,       // Minimum idle connections
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(600),      // 10 minutes
            max_lifetime: Duration::from_secs(1800),     // 30 minutes
            test_on_acquire: true,    // Test connection health before use
        }
    }
}

impl PoolConfig {
    /// Production configuration with higher limits
    pub fn production() -> Self {
        Self {
            max_connections: 50,
            min_connections: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),      // 5 minutes
            max_lifetime: Duration::from_secs(3600),     // 1 hour
            test_on_acquire: true,
        }
    }
    
    /// Development configuration with lower limits
    pub fn development() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(900),      // 15 minutes
            max_lifetime: Duration::from_secs(3600),     // 1 hour
            test_on_acquire: false,
        }
    }
}

/// Create an optimized database connection pool
pub async fn create_pool(database_url: &str, config: PoolConfig) -> Result<PgPool, sqlx::Error> {
    info!(
        "Creating database pool with {} max connections, {} min connections",
        config.max_connections, config.min_connections
    );
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connection_timeout)
        .idle_timeout(Some(config.idle_timeout))
        .max_lifetime(Some(config.max_lifetime))
        .test_before_acquire(config.test_on_acquire)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                // Set session configuration for optimal performance
                use sqlx::Executor;
                
                // Set statement timeout to prevent long-running queries
                conn.execute("SET statement_timeout = '30s'").await?;
                
                // Set lock timeout to prevent indefinite waiting
                conn.execute("SET lock_timeout = '10s'").await?;
                
                // Set idle transaction timeout
                conn.execute("SET idle_in_transaction_session_timeout = '60s'").await?;
                
                // Enable query timing
                conn.execute("SET track_io_timing = ON").await?;
                
                debug!("Database connection configured with optimized settings");
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    
    // Warm up the connection pool
    warm_up_pool(&pool, config.min_connections).await;
    
    info!("Database pool created and warmed up successfully");
    Ok(pool)
}

/// Warm up the connection pool by establishing minimum connections
async fn warm_up_pool(pool: &PgPool, min_connections: u32) {
    debug!("Warming up connection pool with {} connections", min_connections);
    
    let mut handles = Vec::new();
    for i in 0..min_connections {
        let pool = pool.clone();
        let handle = tokio::spawn(async move {
            match pool.acquire().await {
                Ok(conn) => {
                    debug!("Warmed up connection {}", i);
                    // Connection will be returned to pool when dropped
                    drop(conn);
                }
                Err(e) => {
                    warn!("Failed to warm up connection {}: {}", i, e);
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all warmup connections
    for handle in handles {
        let _ = handle.await;
    }
    
    debug!("Connection pool warmup complete");
}

/// Monitor pool health and metrics
pub struct PoolMonitor {
    pool: PgPool,
}

impl PoolMonitor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn start_monitoring(self, interval: Duration) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                self.log_pool_metrics();
            }
        });
    }
    
    fn log_pool_metrics(&self) {
        let size = self.pool.size();
        let idle = self.pool.num_idle();
        let usage = if size > 0 {
            ((size - idle) as f64 / size as f64 * 100.0) as u32
        } else {
            0
        };
        
        debug!(
            "Database pool: {} connections ({} idle, {}% usage)",
            size, idle, usage
        );
        
        // Warn if pool is getting exhausted
        if usage > 80 {
            warn!(
                "Database pool usage is high: {}% ({}/{} connections in use)",
                usage,
                size - idle,
                size
            );
        }
    }
    
    pub fn get_metrics(&self) -> PoolMetrics {
        let size = self.pool.size();
        let idle = self.pool.num_idle();
        
        PoolMetrics {
            total_connections: size,
            idle_connections: idle,
            active_connections: size - idle,
            usage_percentage: if size > 0 {
                ((size - idle) as f64 / size as f64 * 100.0)
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub total_connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
    pub usage_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pool_config_defaults() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 32);
        assert_eq!(config.min_connections, 5);
        assert!(config.test_on_acquire);
    }
    
    #[test]
    fn test_pool_config_production() {
        let config = PoolConfig::production();
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.min_connections, 10);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
    }
    
    #[test]
    fn test_pool_config_development() {
        let config = PoolConfig::development();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert!(!config.test_on_acquire);
    }
}