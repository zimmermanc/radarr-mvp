//! PostgreSQL implementation of MovieRepository with optimized queries
//! Uses JSONB fields for metadata and full-text search capabilities

use crate::database::DatabasePool;
use async_trait::async_trait;
use radarr_core::{
    domain::repositories::MovieRepository,
    models::{MinimumAvailability, Movie, MovieStatus},
    Result,
};
use sqlx::Row;
use uuid::Uuid;

/// Standard movie columns for SELECT queries
const MOVIE_COLUMNS: &str = "id, tmdb_id, imdb_id, title, original_title, year, runtime,
                             status, monitored, quality_profile_id, minimum_availability,
                             has_file, movie_file_id, metadata, alternative_titles,
                             created_at, updated_at, last_search_time, last_info_sync";

/// PostgreSQL implementation of MovieRepository
pub struct PostgresMovieRepository {
    pool: DatabasePool,
}

impl PostgresMovieRepository {
    /// Create a new PostgreSQL movie repository
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    /// Helper function to parse a row into a Movie
    fn parse_movie_from_row(row: &sqlx::postgres::PgRow) -> Result<Movie> {
        Ok(Movie {
            id: row.try_get("id")?,
            tmdb_id: row.try_get("tmdb_id")?,
            imdb_id: row.try_get("imdb_id")?,
            title: row.try_get("title")?,
            original_title: row.try_get("original_title")?,
            year: row.try_get("year")?,
            runtime: row.try_get("runtime")?,
            status: parse_movie_status(&row.try_get::<String, _>("status")?)?,
            monitored: row.try_get("monitored")?,
            quality_profile_id: row.try_get("quality_profile_id")?,
            minimum_availability: parse_minimum_availability(
                &row.try_get::<String, _>("minimum_availability")?,
            )?,
            has_file: row.try_get("has_file")?,
            movie_file_id: row.try_get("movie_file_id")?,
            metadata: row.try_get("metadata")?,
            alternative_titles: row.try_get("alternative_titles")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            last_search_time: row.try_get("last_search_time")?,
            last_info_sync: row.try_get("last_info_sync")?,
        })
    }

    /// Helper method to build a batch insert statement for multiple movies
    /// Uses PostgreSQL's UNNEST function for efficient bulk inserts
    pub async fn create_batch(&self, movies: &[Movie]) -> Result<Vec<Movie>> {
        if movies.is_empty() {
            return Ok(Vec::new());
        }

        // Build arrays for batch insert using PostgreSQL's UNNEST
        let mut ids = Vec::new();
        let mut tmdb_ids = Vec::new();
        let mut imdb_ids = Vec::new();
        let mut titles = Vec::new();
        let mut original_titles = Vec::new();
        let mut years = Vec::new();
        let mut runtimes = Vec::new();
        let mut statuses = Vec::new();
        let mut monitored_flags = Vec::new();
        let mut quality_profile_ids = Vec::new();
        let mut minimum_availabilities = Vec::new();
        let mut has_file_flags = Vec::new();
        let mut movie_file_ids = Vec::new();
        let mut metadatas = Vec::new();
        let mut alternative_titles_list = Vec::new();
        let mut created_ats = Vec::new();
        let mut updated_ats = Vec::new();
        let mut last_search_times = Vec::new();
        let mut last_info_syncs = Vec::new();

        for movie in movies {
            ids.push(movie.id);
            tmdb_ids.push(movie.tmdb_id);
            imdb_ids.push(&movie.imdb_id);
            titles.push(&movie.title);
            original_titles.push(&movie.original_title);
            years.push(movie.year);
            runtimes.push(movie.runtime);
            statuses.push(movie.status.to_string());
            monitored_flags.push(movie.monitored);
            quality_profile_ids.push(movie.quality_profile_id);
            minimum_availabilities.push(movie.minimum_availability.to_string());
            has_file_flags.push(movie.has_file);
            movie_file_ids.push(movie.movie_file_id);
            metadatas.push(&movie.metadata);
            alternative_titles_list.push(&movie.alternative_titles);
            created_ats.push(movie.created_at);
            updated_ats.push(movie.updated_at);
            last_search_times.push(movie.last_search_time);
            last_info_syncs.push(movie.last_info_sync);
        }

        sqlx::query(&format!(
            "INSERT INTO movies ({})
             SELECT * FROM UNNEST($1::uuid[], $2::int[], $3::text[], $4::text[], $5::text[], 
                                  $6::int[], $7::int[], $8::text[], $9::boolean[], $10::int[], 
                                  $11::text[], $12::boolean[], $13::uuid[], $14::jsonb[], $15::jsonb[],
                                  $16::timestamptz[], $17::timestamptz[], $18::timestamptz[], $19::timestamptz[])
             ON CONFLICT (tmdb_id) DO UPDATE SET 
                title = EXCLUDED.title,
                original_title = EXCLUDED.original_title,
                year = EXCLUDED.year,
                runtime = EXCLUDED.runtime,
                status = EXCLUDED.status,
                minimum_availability = EXCLUDED.minimum_availability,
                metadata = EXCLUDED.metadata,
                alternative_titles = EXCLUDED.alternative_titles,
                updated_at = EXCLUDED.updated_at", 
            MOVIE_COLUMNS.replace(", ", ", ")
        ))
        .bind(&ids)
        .bind(&tmdb_ids)
        .bind(&imdb_ids)
        .bind(&titles)
        .bind(&original_titles)
        .bind(&years)
        .bind(&runtimes)
        .bind(&statuses)
        .bind(&monitored_flags)
        .bind(&quality_profile_ids)
        .bind(&minimum_availabilities)
        .bind(&has_file_flags)
        .bind(&movie_file_ids)
        .bind(&metadatas)
        .bind(&alternative_titles_list)
        .bind(&created_ats)
        .bind(&updated_ats)
        .bind(&last_search_times)
        .bind(&last_info_syncs)
        .execute(&self.pool)
        .await?;

        Ok(movies.to_vec())
    }

    /// Find movies by metadata field using JSONB operators
    pub async fn find_by_metadata_field(
        &self,
        field_path: &str,
        value: &serde_json::Value,
    ) -> Result<Vec<Movie>> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM movies 
             WHERE metadata #> $1 = $2
             ORDER BY title ASC",
            MOVIE_COLUMNS
        ))
        .bind(&field_path.split('.').collect::<Vec<_>>())
        .bind(value)
        .fetch_all(&self.pool)
        .await?;

        let mut movies = Vec::new();
        for row in rows {
            movies.push(Self::parse_movie_from_row(&row)?);
        }
        Ok(movies)
    }
}

#[async_trait]
impl MovieRepository for PostgresMovieRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Movie>> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM movies WHERE id = $1",
            MOVIE_COLUMNS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Self::parse_movie_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_tmdb_id(&self, tmdb_id: i32) -> Result<Option<Movie>> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM movies WHERE tmdb_id = $1",
            MOVIE_COLUMNS
        ))
        .bind(tmdb_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Self::parse_movie_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_imdb_id(&self, imdb_id: &str) -> Result<Option<Movie>> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM movies WHERE imdb_id = $1",
            MOVIE_COLUMNS
        ))
        .bind(imdb_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Self::parse_movie_from_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_monitored(&self) -> Result<Vec<Movie>> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM movies WHERE monitored = true ORDER BY title ASC",
            MOVIE_COLUMNS
        ))
        .fetch_all(&self.pool)
        .await?;

        let mut movies = Vec::new();
        for row in rows {
            movies.push(Self::parse_movie_from_row(&row)?);
        }
        Ok(movies)
    }

    async fn find_missing_files(&self) -> Result<Vec<Movie>> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM movies WHERE has_file = false AND monitored = true ORDER BY title ASC",
            MOVIE_COLUMNS
        ))
        .fetch_all(&self.pool)
        .await?;

        let mut movies = Vec::new();
        for row in rows {
            movies.push(Self::parse_movie_from_row(&row)?);
        }
        Ok(movies)
    }

    async fn search_by_title(&self, query: &str, limit: i32) -> Result<Vec<Movie>> {
        let search_query = format!("%{}%", query);
        // PostgreSQL optimized search using ILIKE and JSONB operators
        // This query prioritizes exact title matches, then uses JSONB array search for alternative titles
        let rows = sqlx::query(&format!(
            "SELECT {} FROM movies 
             WHERE title ILIKE $1 
                OR original_title ILIKE $1 
                OR EXISTS (
                    SELECT 1 FROM jsonb_array_elements_text(alternative_titles) AS alt_title
                    WHERE alt_title ILIKE $1
                )
             ORDER BY 
                CASE 
                    WHEN LOWER(title) = LOWER($2) THEN 1
                    WHEN title ILIKE $1 THEN 2
                    WHEN original_title ILIKE $1 THEN 3
                    ELSE 4 
                END,
                ts_rank_cd(to_tsvector('english', title || ' ' || COALESCE(original_title, '')), plainto_tsquery('english', $2)) DESC,
                title ASC
             LIMIT $3", MOVIE_COLUMNS
        ))
        .bind(&search_query)
        .bind(query.trim())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut movies = Vec::new();
        for row in rows {
            movies.push(Self::parse_movie_from_row(&row)?);
        }
        Ok(movies)
    }

    async fn create(&self, movie: &Movie) -> Result<Movie> {
        let _result = sqlx::query(
            "INSERT INTO movies (id, tmdb_id, imdb_id, title, original_title, year, runtime,
             status, monitored, quality_profile_id, minimum_availability,
             has_file, movie_file_id, metadata, alternative_titles,
             created_at, updated_at, last_search_time, last_info_sync)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)"
        )
        .bind(movie.id)
        .bind(movie.tmdb_id)
        .bind(&movie.imdb_id)
        .bind(&movie.title)
        .bind(&movie.original_title)
        .bind(movie.year)
        .bind(movie.runtime)
        .bind(movie.status.to_string())
        .bind(movie.monitored)
        .bind(movie.quality_profile_id)
        .bind(movie.minimum_availability.to_string())
        .bind(movie.has_file)
        .bind(movie.movie_file_id)
        .bind(&movie.metadata)
        .bind(&movie.alternative_titles)
        .bind(movie.created_at)
        .bind(movie.updated_at)
        .bind(movie.last_search_time)
        .bind(movie.last_info_sync)
        .execute(&self.pool)
        .await?;

        Ok(movie.clone())
    }

    async fn update(&self, movie: &Movie) -> Result<Movie> {
        let _result = sqlx::query(
            "UPDATE movies SET tmdb_id = $2, imdb_id = $3, title = $4, original_title = $5,
             year = $6, runtime = $7, status = $8, monitored = $9,
             quality_profile_id = $10, minimum_availability = $11,
             has_file = $12, movie_file_id = $13, metadata = $14,
             alternative_titles = $15, updated_at = $16,
             last_search_time = $17, last_info_sync = $18
             WHERE id = $1",
        )
        .bind(movie.id)
        .bind(movie.tmdb_id)
        .bind(&movie.imdb_id)
        .bind(&movie.title)
        .bind(&movie.original_title)
        .bind(movie.year)
        .bind(movie.runtime)
        .bind(movie.status.to_string())
        .bind(movie.monitored)
        .bind(movie.quality_profile_id)
        .bind(movie.minimum_availability.to_string())
        .bind(movie.has_file)
        .bind(movie.movie_file_id)
        .bind(&movie.metadata)
        .bind(&movie.alternative_titles)
        .bind(movie.updated_at)
        .bind(movie.last_search_time)
        .bind(movie.last_info_sync)
        .execute(&self.pool)
        .await?;

        Ok(movie.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM movies WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list(&self, offset: i64, limit: i32) -> Result<Vec<Movie>> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM movies 
             ORDER BY title ASC
             LIMIT $1 OFFSET $2",
            MOVIE_COLUMNS
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut movies = Vec::new();
        for row in rows {
            movies.push(Self::parse_movie_from_row(&row)?);
        }
        Ok(movies)
    }

    async fn count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM movies")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get::<i64, _>("count").unwrap_or(0))
    }

    async fn update_last_search_time(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE movies SET last_search_time = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn parse_movie_status(status_str: &str) -> Result<MovieStatus> {
    match status_str {
        "announced" => Ok(MovieStatus::Announced),
        "in_production" => Ok(MovieStatus::InProduction),
        "post_production" => Ok(MovieStatus::PostProduction),
        "released" => Ok(MovieStatus::Released),
        "cancelled" => Ok(MovieStatus::Cancelled),
        _ => Err(radarr_core::RadarrError::ValidationError {
            field: "status".to_string(),
            message: format!("Invalid movie status: {}", status_str),
        }),
    }
}

fn parse_minimum_availability(availability_str: &str) -> Result<MinimumAvailability> {
    match availability_str {
        "announced" => Ok(MinimumAvailability::Announced),
        "in_cinemas" => Ok(MinimumAvailability::InCinemas),
        "released" => Ok(MinimumAvailability::Released),
        "predb" => Ok(MinimumAvailability::Predb),
        _ => Err(radarr_core::RadarrError::ValidationError {
            field: "minimum_availability".to_string(),
            message: format!("Invalid minimum availability: {}", availability_str),
        }),
    }
}
