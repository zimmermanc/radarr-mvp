//! Quality domain model

use serde::{Deserialize, Serialize};

/// Quality profile for movie requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProfile {
    pub id: i32,
    pub name: String,
    
    // Quality configuration
    pub cutoff_quality_id: i32,
    pub upgrade_allowed: bool,
    
    // Quality items configuration stored as JSON
    pub items: serde_json::Value,
    
    // Language preferences
    pub language: String,
    
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl QualityProfile {
    /// Create a new quality profile
    pub fn new(name: String, cutoff_quality_id: i32) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id: 0, // Will be set by database
            name,
            cutoff_quality_id,
            upgrade_allowed: true,
            items: serde_json::json!([]),
            language: "english".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Update quality items configuration
    pub fn update_items(&mut self, items: serde_json::Value) {
        self.items = items;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Set language preference
    pub fn set_language(&mut self, language: String) {
        self.language = language;
        self.updated_at = chrono::Utc::now();
    }
}

