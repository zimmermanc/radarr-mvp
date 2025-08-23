//! Blocklist domain models

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use std::fmt;

/// A blocklist entry representing a failed release
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlocklistEntry {
    /// Unique identifier for this blocklist entry
    pub id: Uuid,
    /// The release ID that failed (from releases table or external ID)
    pub release_id: String,
    /// The indexer that provided this release
    pub indexer: String,
    /// The reason why this release was blocked
    pub reason: FailureReason,
    /// When this entry should expire and release can be retried
    pub blocked_until: DateTime<Utc>,
    /// Number of times this release has been retried
    pub retry_count: u32,
    /// Optional movie ID this release was intended for
    pub movie_id: Option<Uuid>,
    /// The title of the blocked release
    pub release_title: String,
    /// When this entry was created
    pub created_at: DateTime<Utc>,
    /// When this entry was last updated
    pub updated_at: DateTime<Utc>,
    /// Additional metadata about the failure
    pub metadata: Option<serde_json::Value>,
}

/// Comprehensive failure reasons for release blocking
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FailureReason {
    /// Connection timed out to indexer or download client
    ConnectionTimeout,
    /// Authentication failed (API key invalid, session expired)
    AuthenticationFailed,
    /// Rate limit exceeded by service
    RateLimited,
    /// Failed to parse response or release information
    ParseError,
    /// Download process stalled or stopped progressing
    DownloadStalled,
    /// Downloaded file hash doesn't match expected
    HashMismatch,
    /// File import process failed (specific reason in metadata)
    ImportFailed(ImportFailureType),
    /// Insufficient disk space for download
    DiskFull,
    /// File system permissions prevent operation
    PermissionDenied,
    /// Release was manually rejected by user
    ManuallyRejected,
    /// Release quality doesn't meet requirements
    QualityRejected,
    /// Release size outside acceptable bounds
    SizeRejected,
    /// Indexer marked release as removed or deleted
    ReleasePurged,
    /// Generic network error (connectivity issues)
    NetworkError,
    /// Service returned 5xx server error
    ServerError,
    /// Malformed or corrupted download
    CorruptedDownload,
    /// Download client returned unexpected error
    DownloadClientError,
    /// Release matches exclusion rules
    ExclusionMatched,
}

/// Specific types of import failures
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImportFailureType {
    /// Failed to move file to destination
    FileMoveError,
    /// Destination file already exists
    FileAlreadyExists,
    /// Unable to create necessary directories
    DirectoryCreationFailed,
    /// File format not recognized or supported
    UnsupportedFormat,
    /// Video quality analysis failed
    QualityAnalysisFailed,
    /// Filename parsing failed
    FilenameParseFailed,
    /// Media information extraction failed
    MediaInfoFailed,
}

impl FailureReason {
    /// Get the default retry delay for this failure type
    pub fn default_retry_delay(&self) -> Duration {
        match self {
            // Authentication and connectivity issues - longer delays
            Self::AuthenticationFailed => Duration::hours(6),
            Self::ConnectionTimeout => Duration::hours(2),
            Self::NetworkError => Duration::hours(1),
            Self::ServerError => Duration::minutes(30),
            
            // Rate limiting - respect typical rate limit windows
            Self::RateLimited => Duration::hours(1),
            
            // Parse and format errors - moderate delay
            Self::ParseError => Duration::hours(4),
            Self::ImportFailed(ImportFailureType::UnsupportedFormat) => Duration::days(1),
            Self::ImportFailed(ImportFailureType::QualityAnalysisFailed) => Duration::hours(6),
            Self::ImportFailed(ImportFailureType::MediaInfoFailed) => Duration::hours(6),
            
            // Resource constraints - shorter delays as they may resolve quickly
            Self::DiskFull => Duration::minutes(30),
            Self::PermissionDenied => Duration::hours(2),
            Self::DownloadClientError => Duration::hours(1),
            
            // Download issues - medium delays
            Self::DownloadStalled => Duration::hours(2),
            Self::HashMismatch => Duration::hours(4),
            Self::CorruptedDownload => Duration::hours(6),
            
            // File system issues - shorter delays for transient issues
            Self::ImportFailed(ImportFailureType::FileMoveError) => Duration::minutes(30),
            Self::ImportFailed(ImportFailureType::FileAlreadyExists) => Duration::days(1),
            Self::ImportFailed(ImportFailureType::DirectoryCreationFailed) => Duration::hours(1),
            Self::ImportFailed(ImportFailureType::FilenameParseFailed) => Duration::hours(12),
            
            // Quality and rule-based rejections - longer delays as unlikely to change
            Self::QualityRejected => Duration::days(7),
            Self::SizeRejected => Duration::days(3),
            Self::ExclusionMatched => Duration::days(30),
            
            // Manual and permanent blocks
            Self::ManuallyRejected => Duration::days(30),
            Self::ReleasePurged => Duration::days(7), // Maybe indexer re-adds
        }
    }
    
    /// Check if this failure type should automatically increase retry delay
    pub fn should_backoff(&self) -> bool {
        matches!(self, 
            Self::ConnectionTimeout
            | Self::NetworkError 
            | Self::ServerError
            | Self::DownloadStalled
            | Self::DownloadClientError
            | Self::ParseError
        )
    }
    
    /// Check if this failure type is considered permanent (no automatic retries)
    pub fn is_permanent(&self) -> bool {
        matches!(self, 
            Self::ManuallyRejected
            | Self::QualityRejected 
            | Self::SizeRejected
            | Self::ExclusionMatched
            | Self::ImportFailed(ImportFailureType::UnsupportedFormat)
            | Self::ImportFailed(ImportFailureType::FileAlreadyExists)
        )
    }
    
    /// Get maximum number of retry attempts for this failure type
    pub fn max_retry_attempts(&self) -> u32 {
        match self {
            // Permanent failures - no retries
            _ if self.is_permanent() => 0,
            
            // Authentication and access issues - few retries
            Self::AuthenticationFailed => 2,
            Self::PermissionDenied => 2,
            
            // Rate limiting - limited retries
            Self::RateLimited => 3,
            
            // Network and connectivity - moderate retries  
            Self::ConnectionTimeout => 5,
            Self::NetworkError => 4,
            Self::ServerError => 3,
            
            // Download issues - more retries as they may be transient
            Self::DownloadStalled => 4,
            Self::DownloadClientError => 4,
            Self::CorruptedDownload => 3,
            Self::HashMismatch => 2,
            
            // Resource constraints - more retries as they resolve over time
            Self::DiskFull => 10,
            
            // Parse and import issues - moderate retries
            Self::ParseError => 3,
            Self::ImportFailed(ImportFailureType::FileMoveError) => 5,
            Self::ImportFailed(ImportFailureType::DirectoryCreationFailed) => 3,
            Self::ImportFailed(ImportFailureType::FilenameParseFailed) => 2,
            Self::ImportFailed(ImportFailureType::QualityAnalysisFailed) => 3,
            Self::ImportFailed(ImportFailureType::MediaInfoFailed) => 3,
            
            // Purged releases - single retry after delay
            Self::ReleasePurged => 1,
            
            // Handle missing import failure types
            Self::ImportFailed(ImportFailureType::UnsupportedFormat) => 0,
            Self::ImportFailed(ImportFailureType::FileAlreadyExists) => 0,
            
            // Handle remaining manual and quality failures
            Self::ManuallyRejected => 0,
            Self::QualityRejected => 0,
            Self::SizeRejected => 0,
            Self::ExclusionMatched => 0,
        }
    }
    
    /// Calculate retry delay with exponential backoff
    pub fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.default_retry_delay();
        
        if self.should_backoff() && attempt > 0 {
            // Apply exponential backoff with jitter
            let multiplier = 2_u64.pow(attempt.min(4)); // Cap at 16x
            let jitter = 0.9 + (attempt as f64 * 0.05); // Small jitter to spread load
            Duration::milliseconds(
                (base_delay.num_milliseconds() as f64 * multiplier as f64 * jitter) as i64
            )
        } else {
            base_delay
        }
    }
    
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::ConnectionTimeout => "Connection timed out",
            Self::AuthenticationFailed => "Authentication failed",
            Self::RateLimited => "Rate limit exceeded",
            Self::ParseError => "Failed to parse response",
            Self::DownloadStalled => "Download stalled",
            Self::HashMismatch => "Download hash mismatch",
            Self::ImportFailed(ImportFailureType::FileMoveError) => "Failed to move file",
            Self::ImportFailed(ImportFailureType::FileAlreadyExists) => "File already exists",
            Self::ImportFailed(ImportFailureType::DirectoryCreationFailed) => "Failed to create directory",
            Self::ImportFailed(ImportFailureType::UnsupportedFormat) => "Unsupported file format",
            Self::ImportFailed(ImportFailureType::QualityAnalysisFailed) => "Quality analysis failed",
            Self::ImportFailed(ImportFailureType::FilenameParseFailed) => "Filename parse failed",
            Self::ImportFailed(ImportFailureType::MediaInfoFailed) => "Media info extraction failed",
            Self::DiskFull => "Insufficient disk space",
            Self::PermissionDenied => "Permission denied",
            Self::ManuallyRejected => "Manually rejected",
            Self::QualityRejected => "Quality requirements not met",
            Self::SizeRejected => "File size rejected",
            Self::ReleasePurged => "Release removed from indexer",
            Self::NetworkError => "Network error",
            Self::ServerError => "Server error",
            Self::CorruptedDownload => "Downloaded file corrupted",
            Self::DownloadClientError => "Download client error",
            Self::ExclusionMatched => "Matched exclusion rule",
        }
    }
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl fmt::Display for ImportFailureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileMoveError => write!(f, "FileMoveError"),
            Self::FileAlreadyExists => write!(f, "FileAlreadyExists"),
            Self::DirectoryCreationFailed => write!(f, "DirectoryCreationFailed"),
            Self::UnsupportedFormat => write!(f, "UnsupportedFormat"),
            Self::QualityAnalysisFailed => write!(f, "QualityAnalysisFailed"),
            Self::FilenameParseFailed => write!(f, "FilenameParseFailed"),
            Self::MediaInfoFailed => write!(f, "MediaInfoFailed"),
        }
    }
}

impl BlocklistEntry {
    /// Create a new blocklist entry
    pub fn new(
        release_id: String,
        indexer: String,
        reason: FailureReason,
        release_title: String,
    ) -> Self {
        let now = Utc::now();
        let retry_delay = reason.calculate_retry_delay(0);
        
        Self {
            id: Uuid::new_v4(),
            release_id,
            indexer,
            reason,
            blocked_until: now + retry_delay,
            retry_count: 0,
            movie_id: None,
            release_title,
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }
    
    /// Create a blocklist entry for a specific movie
    pub fn new_for_movie(
        release_id: String,
        indexer: String,
        reason: FailureReason,
        release_title: String,
        movie_id: Uuid,
    ) -> Self {
        let mut entry = Self::new(release_id, indexer, reason, release_title);
        entry.movie_id = Some(movie_id);
        entry
    }
    
    /// Check if this entry has expired and can be retried
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.blocked_until
    }
    
    /// Check if this release can be retried (not permanent and hasn't exceeded max attempts)
    pub fn can_retry(&self) -> bool {
        !self.reason.is_permanent() && self.retry_count < self.reason.max_retry_attempts()
    }
    
    /// Update this entry for a retry attempt
    pub fn retry(&mut self) -> bool {
        if !self.can_retry() {
            return false;
        }
        
        self.retry_count += 1;
        let retry_delay = self.reason.calculate_retry_delay(self.retry_count);
        self.blocked_until = Utc::now() + retry_delay;
        self.updated_at = Utc::now();
        
        true
    }
    
    /// Add metadata to this entry
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// Get the time remaining until this entry expires
    pub fn time_until_expiry(&self) -> Option<Duration> {
        let now = Utc::now();
        if self.blocked_until > now {
            Some(self.blocked_until - now)
        } else {
            None
        }
    }
    
    /// Check if this is a permanent block (manual rejection or permanent failure)
    pub fn is_permanent_block(&self) -> bool {
        self.reason.is_permanent() || self.retry_count >= self.reason.max_retry_attempts()
    }
}

/// Query parameters for searching blocklist entries
#[derive(Debug, Clone, Default)]
pub struct BlocklistQuery {
    /// Filter by indexer name
    pub indexer: Option<String>,
    /// Filter by failure reason
    pub reason: Option<FailureReason>,
    /// Filter by movie ID
    pub movie_id: Option<Uuid>,
    /// Include only expired entries
    pub expired_only: bool,
    /// Include only active (non-expired) entries
    pub active_only: bool,
    /// Pagination offset
    pub offset: i64,
    /// Maximum number of results
    pub limit: i32,
}

impl BlocklistQuery {
    /// Create a new query for expired entries
    pub fn expired() -> Self {
        Self {
            expired_only: true,
            ..Default::default()
        }
    }
    
    /// Create a new query for active (non-expired) entries
    pub fn active() -> Self {
        Self {
            active_only: true,
            ..Default::default()
        }
    }
    
    /// Filter by indexer
    pub fn with_indexer(mut self, indexer: impl Into<String>) -> Self {
        self.indexer = Some(indexer.into());
        self
    }
    
    /// Filter by failure reason
    pub fn with_reason(mut self, reason: FailureReason) -> Self {
        self.reason = Some(reason);
        self
    }
    
    /// Filter by movie
    pub fn with_movie_id(mut self, movie_id: Uuid) -> Self {
        self.movie_id = Some(movie_id);
        self
    }
    
    /// Set pagination
    pub fn paginate(mut self, offset: i64, limit: i32) -> Self {
        self.offset = offset;
        self.limit = limit;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_reason_retry_delays() {
        assert_eq!(FailureReason::AuthenticationFailed.default_retry_delay(), Duration::hours(6));
        assert_eq!(FailureReason::DiskFull.default_retry_delay(), Duration::minutes(30));
        assert_eq!(FailureReason::QualityRejected.default_retry_delay(), Duration::days(7));
    }
    
    #[test]
    fn test_failure_reason_permanence() {
        assert!(FailureReason::ManuallyRejected.is_permanent());
        assert!(FailureReason::QualityRejected.is_permanent());
        assert!(!FailureReason::ConnectionTimeout.is_permanent());
        assert!(!FailureReason::DiskFull.is_permanent());
    }
    
    #[test]
    fn test_failure_reason_backoff() {
        assert!(FailureReason::ConnectionTimeout.should_backoff());
        assert!(FailureReason::NetworkError.should_backoff());
        assert!(!FailureReason::ManuallyRejected.should_backoff());
        assert!(!FailureReason::QualityRejected.should_backoff());
    }
    
    #[test]
    fn test_exponential_backoff() {
        let reason = FailureReason::ConnectionTimeout;
        let base_delay = reason.default_retry_delay();
        
        let delay_0 = reason.calculate_retry_delay(0);
        let delay_1 = reason.calculate_retry_delay(1);
        let delay_2 = reason.calculate_retry_delay(2);
        
        assert_eq!(delay_0, base_delay);
        assert!(delay_1 > delay_0);
        assert!(delay_2 > delay_1);
    }
    
    #[test]
    fn test_blocklist_entry_creation() {
        let entry = BlocklistEntry::new(
            "test-release-123".to_string(),
            "test-indexer".to_string(),
            FailureReason::ConnectionTimeout,
            "Test Release".to_string(),
        );
        
        assert_eq!(entry.release_id, "test-release-123");
        assert_eq!(entry.indexer, "test-indexer");
        assert_eq!(entry.reason, FailureReason::ConnectionTimeout);
        assert_eq!(entry.retry_count, 0);
        assert!(entry.blocked_until > Utc::now());
        assert!(entry.can_retry());
        assert!(!entry.is_expired());
    }
    
    #[test]
    fn test_blocklist_entry_retry() {
        let mut entry = BlocklistEntry::new(
            "test-release-123".to_string(),
            "test-indexer".to_string(),
            FailureReason::ConnectionTimeout,
            "Test Release".to_string(),
        );
        
        let original_blocked_until = entry.blocked_until;
        let success = entry.retry();
        
        assert!(success);
        assert_eq!(entry.retry_count, 1);
        assert!(entry.blocked_until > original_blocked_until);
    }
    
    #[test]
    fn test_permanent_block_no_retry() {
        let mut entry = BlocklistEntry::new(
            "test-release-123".to_string(),
            "test-indexer".to_string(),
            FailureReason::ManuallyRejected,
            "Test Release".to_string(),
        );
        
        assert!(!entry.can_retry());
        let success = entry.retry();
        assert!(!success);
        assert_eq!(entry.retry_count, 0);
    }
    
    #[test]
    fn test_max_retries_exceeded() {
        let mut entry = BlocklistEntry::new(
            "test-release-123".to_string(),
            "test-indexer".to_string(),
            FailureReason::AuthenticationFailed, // max 2 retries
            "Test Release".to_string(),
        );
        
        // First retry should work
        assert!(entry.retry());
        assert_eq!(entry.retry_count, 1);
        assert!(entry.can_retry());
        
        // Second retry should work
        assert!(entry.retry());
        assert_eq!(entry.retry_count, 2);
        assert!(!entry.can_retry()); // Should now exceed max
        
        // Third retry should fail
        assert!(!entry.retry());
        assert_eq!(entry.retry_count, 2);
    }
    
    #[test]
    fn test_blocklist_query_builder() {
        let query = BlocklistQuery::active()
            .with_indexer("test-indexer")
            .with_reason(FailureReason::ConnectionTimeout)
            .paginate(10, 20);
        
        assert!(query.active_only);
        assert!(!query.expired_only);
        assert_eq!(query.indexer, Some("test-indexer".to_string()));
        assert_eq!(query.reason, Some(FailureReason::ConnectionTimeout));
        assert_eq!(query.offset, 10);
        assert_eq!(query.limit, 20);
    }
}