//! PostgreSQL implementation of CustomFormatsRepository

use crate::database::DatabasePool;
use async_trait::async_trait;
use radarr_core::{RadarrError, Result};
use radarr_decision::{CustomFormat, FormatSpecification};
use sqlx::Row;
use uuid::Uuid;

/// Repository trait for custom formats
#[async_trait]
pub trait CustomFormatsRepository: Send + Sync {
    /// Find custom format by ID
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<CustomFormat>>;

    /// Find custom format by name
    async fn find_by_name(&self, name: &str) -> Result<Option<CustomFormat>>;

    /// Create a new custom format
    async fn create(&self, format: &CustomFormat) -> Result<CustomFormat>;

    /// Update an existing custom format
    async fn update(&self, format: &CustomFormat) -> Result<CustomFormat>;

    /// Delete a custom format
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// List all custom formats
    async fn list(&self) -> Result<Vec<CustomFormat>>;

    /// List only enabled custom formats
    async fn list_enabled(&self) -> Result<Vec<CustomFormat>>;
}

/// PostgreSQL implementation of CustomFormatsRepository
pub struct PostgresCustomFormatsRepository {
    pool: DatabasePool,
}

impl PostgresCustomFormatsRepository {
    /// Create a new PostgreSQL custom formats repository
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    /// Convert database row to CustomFormat
    fn row_to_custom_format(&self, row: &sqlx::postgres::PgRow) -> Result<CustomFormat> {
        let id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let specifications_json: serde_json::Value = row.try_get("specifications")?;
        let score: i32 = row.try_get("score")?;
        let enabled: bool = row.try_get("enabled")?;

        // Parse specifications from JSON
        let specifications: Vec<FormatSpecification> = serde_json::from_value(specifications_json)
            .map_err(|e| {
                RadarrError::SerializationError(format!("Failed to parse specifications: {}", e))
            })?;

        Ok(CustomFormat {
            id,
            name,
            specifications,
            score,
            enabled,
        })
    }
}

#[async_trait]
impl CustomFormatsRepository for PostgresCustomFormatsRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<CustomFormat>> {
        let row = sqlx::query(
            "SELECT id, name, specifications, score, enabled, created_at, updated_at 
             FROM custom_formats WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_custom_format(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<CustomFormat>> {
        let row = sqlx::query(
            "SELECT id, name, specifications, score, enabled, created_at, updated_at 
             FROM custom_formats WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_custom_format(&row)?)),
            None => Ok(None),
        }
    }

    async fn create(&self, format: &CustomFormat) -> Result<CustomFormat> {
        let specifications_json = serde_json::to_value(&format.specifications).map_err(|e| {
            RadarrError::SerializationError(format!("Failed to serialize specifications: {}", e))
        })?;

        sqlx::query(
            "INSERT INTO custom_formats (id, name, specifications, score, enabled, created_at, updated_at) 
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
        )
        .bind(&format.id)
        .bind(&format.name)
        .bind(&specifications_json)
        .bind(format.score)
        .bind(format.enabled)
        .execute(&self.pool)
        .await?;

        Ok(format.clone())
    }

    async fn update(&self, format: &CustomFormat) -> Result<CustomFormat> {
        let specifications_json = serde_json::to_value(&format.specifications).map_err(|e| {
            RadarrError::SerializationError(format!("Failed to serialize specifications: {}", e))
        })?;

        sqlx::query(
            "UPDATE custom_formats 
             SET name = $2, specifications = $3, score = $4, enabled = $5, updated_at = NOW()
             WHERE id = $1",
        )
        .bind(&format.id)
        .bind(&format.name)
        .bind(&specifications_json)
        .bind(format.score)
        .bind(format.enabled)
        .execute(&self.pool)
        .await?;

        Ok(format.clone())
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        sqlx::query("DELETE FROM custom_formats WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<CustomFormat>> {
        let rows = sqlx::query(
            "SELECT id, name, specifications, score, enabled, created_at, updated_at 
             FROM custom_formats ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut formats = Vec::new();
        for row in rows {
            formats.push(self.row_to_custom_format(&row)?);
        }
        Ok(formats)
    }

    async fn list_enabled(&self) -> Result<Vec<CustomFormat>> {
        let rows = sqlx::query(
            "SELECT id, name, specifications, score, enabled, created_at, updated_at 
             FROM custom_formats WHERE enabled = true ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut formats = Vec::new();
        for row in rows {
            formats.push(self.row_to_custom_format(&row)?);
        }
        Ok(formats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radarr_decision::FormatSpecification;

    #[tokio::test]
    async fn test_custom_format_serialization() {
        let spec = FormatSpecification::new("release_title", "x265|HEVC");
        let format = CustomFormat::new("x265 Format", 5).add_spec(spec);

        // Test that we can serialize/deserialize specifications
        let json = serde_json::to_value(&format.specifications).unwrap();
        let deserialized: Vec<FormatSpecification> = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].spec_type, "release_title");
        assert_eq!(deserialized[0].value, "x265|HEVC");
    }
}
