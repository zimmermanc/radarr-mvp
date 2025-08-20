//! Simplified PostgreSQL implementation of QualityProfileRepository

use crate::database::DatabasePool;
use async_trait::async_trait;
use radarr_core::{
    domain::repositories::QualityProfileRepository,
    models::QualityProfile,
    Result,
};
use sqlx::Row;

/// PostgreSQL implementation of QualityProfileRepository
pub struct PostgresQualityProfileRepository {
    pool: DatabasePool,
}

impl PostgresQualityProfileRepository {
    /// Create a new PostgreSQL quality profile repository
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QualityProfileRepository for PostgresQualityProfileRepository {
    async fn find_by_id(&self, id: i32) -> Result<Option<QualityProfile>> {
        let row = sqlx::query(
            "SELECT id, name, cutoff_quality_id, upgrade_allowed, items, language,
             created_at, updated_at FROM quality_profiles WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let profile = QualityProfile {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    cutoff_quality_id: row.try_get("cutoff_quality_id")?,
                    upgrade_allowed: row.try_get("upgrade_allowed")?,
                    items: row.try_get("items")?,
                    language: row.try_get("language")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<QualityProfile>> {
        let row = sqlx::query(
            "SELECT id, name, cutoff_quality_id, upgrade_allowed, items, language,
             created_at, updated_at FROM quality_profiles WHERE name = $1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let profile = QualityProfile {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    cutoff_quality_id: row.try_get("cutoff_quality_id")?,
                    upgrade_allowed: row.try_get("upgrade_allowed")?,
                    items: row.try_get("items")?,
                    language: row.try_get("language")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }

    async fn create(&self, profile: &QualityProfile) -> Result<QualityProfile> {
        let _result = sqlx::query(
            "INSERT INTO quality_profiles (name, cutoff_quality_id, upgrade_allowed, items, language,
             created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(&profile.name)
        .bind(profile.cutoff_quality_id)
        .bind(profile.upgrade_allowed)
        .bind(&profile.items)
        .bind(&profile.language)
        .bind(profile.created_at)
        .bind(profile.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(profile.clone())
    }

    async fn update(&self, profile: &QualityProfile) -> Result<QualityProfile> {
        let _result = sqlx::query(
            "UPDATE quality_profiles SET name = $2, cutoff_quality_id = $3, upgrade_allowed = $4,
             items = $5, language = $6, updated_at = $7 WHERE id = $1"
        )
        .bind(profile.id)
        .bind(&profile.name)
        .bind(profile.cutoff_quality_id)
        .bind(profile.upgrade_allowed)
        .bind(&profile.items)
        .bind(&profile.language)
        .bind(profile.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(profile.clone())
    }

    async fn delete(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM quality_profiles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<QualityProfile>> {
        let rows = sqlx::query(
            "SELECT id, name, cutoff_quality_id, upgrade_allowed, items, language,
             created_at, updated_at FROM quality_profiles ORDER BY name ASC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut profiles = Vec::new();
        for row in rows {
            let profile = QualityProfile {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                cutoff_quality_id: row.try_get("cutoff_quality_id")?,
                upgrade_allowed: row.try_get("upgrade_allowed")?,
                items: row.try_get("items")?,
                language: row.try_get("language")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            profiles.push(profile);
        }
        Ok(profiles)
    }

    async fn get_default(&self) -> Result<Option<QualityProfile>> {
        // Look for a profile named 'Default' first, then fall back to the first profile
        let row = sqlx::query(
            "SELECT id, name, cutoff_quality_id, upgrade_allowed, items, language,
             created_at, updated_at FROM quality_profiles 
             WHERE name ILIKE 'default%' OR name ILIKE '%default%'
             ORDER BY 
                CASE WHEN LOWER(name) = 'default' THEN 1 ELSE 2 END,
                id ASC
             LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let profile = QualityProfile {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    cutoff_quality_id: row.try_get("cutoff_quality_id")?,
                    upgrade_allowed: row.try_get("upgrade_allowed")?,
                    items: row.try_get("items")?,
                    language: row.try_get("language")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(profile))
            }
            None => {
                // If no default found, return the first profile available
                let row = sqlx::query(
                    "SELECT id, name, cutoff_quality_id, upgrade_allowed, items, language,
                     created_at, updated_at FROM quality_profiles ORDER BY id ASC LIMIT 1"
                )
                .fetch_optional(&self.pool)
                .await?;

                match row {
                    Some(row) => {
                        let profile = QualityProfile {
                            id: row.try_get("id")?,
                            name: row.try_get("name")?,
                            cutoff_quality_id: row.try_get("cutoff_quality_id")?,
                            upgrade_allowed: row.try_get("upgrade_allowed")?,
                            items: row.try_get("items")?,
                            language: row.try_get("language")?,
                            created_at: row.try_get("created_at")?,
                            updated_at: row.try_get("updated_at")?,
                        };
                        Ok(Some(profile))
                    }
                    None => Ok(None),
                }
            }
        }
    }
}