//! Simplified PostgreSQL implementation of IndexerRepository

use crate::database::DatabasePool;
use async_trait::async_trait;
use radarr_core::{
    domain::repositories::IndexerRepository,
    models::{Indexer, IndexerImplementation},
    Result,
};
use sqlx::Row;

/// PostgreSQL implementation of IndexerRepository
pub struct PostgresIndexerRepository {
    pool: DatabasePool,
}

impl PostgresIndexerRepository {
    /// Create a new PostgreSQL indexer repository
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IndexerRepository for PostgresIndexerRepository {
    async fn find_by_id(&self, id: i32) -> Result<Option<Indexer>> {
        let row = sqlx::query(
            "SELECT id, name, implementation, settings, enabled, priority,
             enable_rss, enable_automatic_search, enable_interactive_search,
             download_client_id, created_at, updated_at
             FROM indexers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let indexer = Indexer {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    implementation: parse_indexer_implementation(
                        &row.try_get::<String, _>("implementation")?,
                    )?,
                    settings: row.try_get("settings")?,
                    enabled: row.try_get("enabled")?,
                    priority: row.try_get("priority")?,
                    enable_rss: row.try_get("enable_rss")?,
                    enable_automatic_search: row.try_get("enable_automatic_search")?,
                    enable_interactive_search: row.try_get("enable_interactive_search")?,
                    download_client_id: row.try_get("download_client_id")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(indexer))
            }
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Indexer>> {
        let row = sqlx::query(
            "SELECT id, name, implementation, settings, enabled, priority,
             enable_rss, enable_automatic_search, enable_interactive_search,
             download_client_id, created_at, updated_at
             FROM indexers WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let indexer = Indexer {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    implementation: parse_indexer_implementation(
                        &row.try_get::<String, _>("implementation")?,
                    )?,
                    settings: row.try_get("settings")?,
                    enabled: row.try_get("enabled")?,
                    priority: row.try_get("priority")?,
                    enable_rss: row.try_get("enable_rss")?,
                    enable_automatic_search: row.try_get("enable_automatic_search")?,
                    enable_interactive_search: row.try_get("enable_interactive_search")?,
                    download_client_id: row.try_get("download_client_id")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(indexer))
            }
            None => Ok(None),
        }
    }

    async fn find_enabled(&self) -> Result<Vec<Indexer>> {
        let rows = sqlx::query(
            "SELECT id, name, implementation, settings, enabled, priority,
             enable_rss, enable_automatic_search, enable_interactive_search,
             download_client_id, created_at, updated_at
             FROM indexers WHERE enabled = true ORDER BY priority ASC, name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut indexers = Vec::new();
        for row in rows {
            let indexer = Indexer {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                implementation: parse_indexer_implementation(
                    &row.try_get::<String, _>("implementation")?,
                )?,
                settings: row.try_get("settings")?,
                enabled: row.try_get("enabled")?,
                priority: row.try_get("priority")?,
                enable_rss: row.try_get("enable_rss")?,
                enable_automatic_search: row.try_get("enable_automatic_search")?,
                enable_interactive_search: row.try_get("enable_interactive_search")?,
                download_client_id: row.try_get("download_client_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            indexers.push(indexer);
        }
        Ok(indexers)
    }

    async fn create(&self, indexer: &Indexer) -> Result<Indexer> {
        let _result = sqlx::query(
            "INSERT INTO indexers (name, implementation, settings, enabled, priority,
             enable_rss, enable_automatic_search, enable_interactive_search,
             download_client_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&indexer.name)
        .bind(indexer.implementation.to_string())
        .bind(&indexer.settings)
        .bind(indexer.enabled)
        .bind(indexer.priority)
        .bind(indexer.enable_rss)
        .bind(indexer.enable_automatic_search)
        .bind(indexer.enable_interactive_search)
        .bind(indexer.download_client_id)
        .bind(indexer.created_at)
        .bind(indexer.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(indexer.clone())
    }

    async fn update(&self, indexer: &Indexer) -> Result<Indexer> {
        let _result = sqlx::query(
            "UPDATE indexers SET name = $2, implementation = $3, settings = $4, enabled = $5,
             priority = $6, enable_rss = $7, enable_automatic_search = $8,
             enable_interactive_search = $9, download_client_id = $10, updated_at = $11
             WHERE id = $1",
        )
        .bind(indexer.id)
        .bind(&indexer.name)
        .bind(indexer.implementation.to_string())
        .bind(&indexer.settings)
        .bind(indexer.enabled)
        .bind(indexer.priority)
        .bind(indexer.enable_rss)
        .bind(indexer.enable_automatic_search)
        .bind(indexer.enable_interactive_search)
        .bind(indexer.download_client_id)
        .bind(indexer.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(indexer.clone())
    }

    async fn delete(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM indexers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Indexer>> {
        let rows = sqlx::query(
            "SELECT id, name, implementation, settings, enabled, priority,
             enable_rss, enable_automatic_search, enable_interactive_search,
             download_client_id, created_at, updated_at
             FROM indexers ORDER BY priority ASC, name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut indexers = Vec::new();
        for row in rows {
            let indexer = Indexer {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                implementation: parse_indexer_implementation(
                    &row.try_get::<String, _>("implementation")?,
                )?,
                settings: row.try_get("settings")?,
                enabled: row.try_get("enabled")?,
                priority: row.try_get("priority")?,
                enable_rss: row.try_get("enable_rss")?,
                enable_automatic_search: row.try_get("enable_automatic_search")?,
                enable_interactive_search: row.try_get("enable_interactive_search")?,
                download_client_id: row.try_get("download_client_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            indexers.push(indexer);
        }
        Ok(indexers)
    }

    async fn test_connection(&self, id: i32) -> Result<bool> {
        // Check if indexer exists and is configured
        let row = sqlx::query("SELECT enabled, settings FROM indexers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let enabled: bool = row.try_get("enabled")?;
                let settings: serde_json::Value = row.try_get("settings")?;

                // Basic validation - check if enabled and has required settings
                Ok(enabled
                    && !settings.is_null()
                    && settings.as_object().map_or(false, |obj| !obj.is_empty()))
            }
            None => Ok(false),
        }
    }
}

fn parse_indexer_implementation(implementation_str: &str) -> Result<IndexerImplementation> {
    match implementation_str {
        "prowlarr" => Ok(IndexerImplementation::Prowlarr),
        "jackett" => Ok(IndexerImplementation::Jackett),
        "torznab" => Ok(IndexerImplementation::Torznab),
        "newznab" => Ok(IndexerImplementation::Newznab),
        _ => Err(radarr_core::RadarrError::ValidationError {
            field: "implementation".to_string(),
            message: format!("Invalid indexer implementation: {}", implementation_str),
        }),
    }
}
