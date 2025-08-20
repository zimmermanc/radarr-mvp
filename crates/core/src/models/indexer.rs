//! Indexer domain model

use serde::{Deserialize, Serialize};

/// Indexer implementation type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexerImplementation {
    Prowlarr,
    Jackett,
    Torznab,
    Newznab,
}

/// Indexer configuration and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indexer {
    pub id: i32,
    pub name: String,
    pub implementation: IndexerImplementation,
    
    // Configuration settings stored as JSON
    pub settings: serde_json::Value,
    
    // Indexer behavior flags
    pub enabled: bool,
    pub priority: i32,
    pub enable_rss: bool,
    pub enable_automatic_search: bool,
    pub enable_interactive_search: bool,
    
    // Download client association
    pub download_client_id: Option<i32>,
    
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Indexer {
    /// Create a new indexer with the given name and implementation
    pub fn new(name: String, implementation: IndexerImplementation) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id: 0, // Will be set by database
            name,
            implementation,
            settings: serde_json::json!({}),
            enabled: true,
            priority: 25,
            enable_rss: true,
            enable_automatic_search: true,
            enable_interactive_search: true,
            download_client_id: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Update indexer settings
    pub fn update_settings(&mut self, settings: serde_json::Value) {
        self.settings = settings;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Enable or disable the indexer
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Get the base URL from settings
    pub fn base_url(&self) -> Option<&str> {
        self.settings.get("base_url").and_then(|url| url.as_str())
    }
    
    /// Get the API key from settings
    pub fn api_key(&self) -> Option<&str> {
        self.settings.get("api_key").and_then(|key| key.as_str())
    }
}

// Implement Display for enum serialization to string
impl std::fmt::Display for IndexerImplementation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexerImplementation::Prowlarr => write!(f, "prowlarr"),
            IndexerImplementation::Jackett => write!(f, "jackett"),
            IndexerImplementation::Torznab => write!(f, "torznab"),
            IndexerImplementation::Newznab => write!(f, "newznab"),
        }
    }
}
