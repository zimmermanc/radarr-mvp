//! Repository implementations
//!
//! This module contains PostgreSQL implementations of all repository traits
//! defined in the core domain layer.

pub mod movie;
pub mod indexer;
pub mod quality_profile;
pub mod download;
pub mod queue;

// Re-export all repository implementations
pub use movie::PostgresMovieRepository;
pub use indexer::PostgresIndexerRepository;
pub use quality_profile::PostgresQualityProfileRepository;
pub use download::PostgresDownloadRepository;
pub use queue::PostgresQueueRepository;

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
        let _quality_profile_repo_type = std::marker::PhantomData::<PostgresQualityProfileRepository>;
        let _download_repo_type = std::marker::PhantomData::<PostgresDownloadRepository>;
    }
}