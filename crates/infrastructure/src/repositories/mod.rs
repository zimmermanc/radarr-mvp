//! Repository implementations
//!
//! This module contains PostgreSQL implementations of all repository traits
//! defined in the core domain layer.

pub mod custom_formats;
pub mod download;
pub mod indexer;
pub mod movie;
pub mod quality_profile;
pub mod queue;
pub mod streaming_cache;
// pub mod list_sync; // Temporarily disabled - has SQLX type issues
pub mod blocklist;

// Re-export all repository implementations
pub use custom_formats::{CustomFormatsRepository, PostgresCustomFormatsRepository};
pub use download::PostgresDownloadRepository;
pub use indexer::PostgresIndexerRepository;
pub use movie::PostgresMovieRepository;
pub use quality_profile::PostgresQualityProfileRepository;
pub use queue::PostgresQueueRepository;
pub use streaming_cache::PostgresStreamingCache;
// pub use list_sync::PostgresListSyncRepository; // Temporarily disabled
pub use blocklist::PostgresBlocklistRepository;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_struct_creation() {
        // This is a basic test to ensure the repository structs can be created
        // In a real test, we would need a database connection pool

        // For now, just test that the types exist and can be imported
        let _movie_repo_type = std::marker::PhantomData::<PostgresMovieRepository>;
        let _indexer_repo_type = std::marker::PhantomData::<PostgresIndexerRepository>;
        let _quality_profile_repo_type =
            std::marker::PhantomData::<PostgresQualityProfileRepository>;
        let _download_repo_type = std::marker::PhantomData::<PostgresDownloadRepository>;
    }
}
