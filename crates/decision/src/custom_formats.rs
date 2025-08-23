//! Custom Formats for advanced quality filtering and scoring
//!
//! Custom formats allow users to define specific rules for identifying
//! and scoring releases based on various criteria like codecs, groups,
//! special features, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use regex::Regex;

/// Specification for a custom format rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatSpecification {
    /// Type of specification (release_title, indexer_flag, size, etc.)
    pub spec_type: String,
    /// Whether to negate this rule
    pub negate: bool,
    /// Whether this specification is required
    pub required: bool,
    /// The value/pattern to match
    pub value: String,
}

impl FormatSpecification {
    /// Create a new format specification
    pub fn new(spec_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            spec_type: spec_type.into(),
            negate: false,
            required: false,
            value: value.into(),
        }
    }

    /// Set whether this specification should be negated
    pub fn negate(mut self, negate: bool) -> Self {
        self.negate = negate;
        self
    }

    /// Set whether this specification is required
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Check if this specification matches the given release data
    pub fn matches(&self, release_data: &ReleaseData) -> bool {
        let result = match self.spec_type.as_str() {
            "release_title" => self.matches_title(&release_data.title),
            "indexer_flag" => self.matches_indexer_flag(release_data),
            "size" => self.matches_size(release_data),
            "seeders" => self.matches_seeders(release_data),
            "release_group" => self.matches_release_group(&release_data.title),
            "codec" => self.matches_codec(&release_data.title),
            "source" => self.matches_source(&release_data.title),
            _ => false,
        };

        if self.negate { !result } else { result }
    }

    /// Match against release title using regex
    fn matches_title(&self, title: &str) -> bool {
        if let Ok(regex) = Regex::new(&format!("(?i){}", self.value)) {
            regex.is_match(title)
        } else {
            title.to_lowercase().contains(&self.value.to_lowercase())
        }
    }

    /// Match against indexer flags (freeleech, internal, etc.)
    fn matches_indexer_flag(&self, release_data: &ReleaseData) -> bool {
        match self.value.as_str() {
            "freeleech" => release_data.freeleech.unwrap_or(false),
            "internal" => release_data.internal.unwrap_or(false),
            _ => false,
        }
    }

    /// Match against file size
    fn matches_size(&self, release_data: &ReleaseData) -> bool {
        if let Some(size) = release_data.size_bytes {
            if let Ok(condition) = self.parse_size_condition(&self.value) {
                return condition.evaluate(size as i64);
            }
        }
        false
    }

    /// Match against seeder count
    fn matches_seeders(&self, release_data: &ReleaseData) -> bool {
        if let Some(seeders) = release_data.seeders {
            if let Ok(condition) = self.parse_numeric_condition(&self.value) {
                return condition.evaluate(seeders as i64);
            }
        }
        false
    }

    /// Match against release group
    fn matches_release_group(&self, title: &str) -> bool {
        // Extract release group (usually after the last '-')
        if let Some(group) = title.split('-').last() {
            self.matches_title(group.trim())
        } else {
            false
        }
    }

    /// Match against video codec
    fn matches_codec(&self, title: &str) -> bool {
        let title_lower = title.to_lowercase();
        let value_lower = self.value.to_lowercase();
        
        match value_lower.as_str() {
            "x264" => title_lower.contains("x264") || title_lower.contains("h264"),
            "x265" => title_lower.contains("x265") || title_lower.contains("h265") || title_lower.contains("hevc"),
            "xvid" => title_lower.contains("xvid"),
            _ => self.matches_title(title),
        }
    }

    /// Match against source type
    fn matches_source(&self, title: &str) -> bool {
        let title_lower = title.to_lowercase();
        let value_lower = self.value.to_lowercase();
        
        match value_lower.as_str() {
            "bluray" => title_lower.contains("bluray") || title_lower.contains("blu-ray") || title_lower.contains("bdmv"),
            "webdl" => title_lower.contains("web-dl") || title_lower.contains("webdl"),
            "webrip" => title_lower.contains("webrip") || title_lower.contains("web-rip"),
            "hdtv" => title_lower.contains("hdtv"),
            "dvd" => title_lower.contains("dvd") && !title_lower.contains("hddvd"),
            "remux" => title_lower.contains("remux"),
            _ => self.matches_title(title),
        }
    }

    /// Parse size condition (e.g., ">5GB", "<=10GB")
    fn parse_size_condition(&self, condition: &str) -> Result<NumericCondition, String> {
        NumericCondition::parse_size(condition)
    }

    /// Parse numeric condition (e.g., ">=10", "<5")  
    fn parse_numeric_condition(&self, condition: &str) -> Result<NumericCondition, String> {
        NumericCondition::parse(condition)
    }
}

/// Numeric condition for size/seeders matching
#[derive(Debug, Clone)]
pub struct NumericCondition {
    pub operator: String,
    pub value: i64,
    pub unit: Option<String>,
}

impl NumericCondition {
    /// Parse a numeric condition string
    pub fn parse(condition: &str) -> Result<Self, String> {
        let condition = condition.trim();
        
        let (operator, value_str) = if condition.starts_with(">=") {
            (">=", &condition[2..])
        } else if condition.starts_with("<=") {
            ("<=", &condition[2..])
        } else if condition.starts_with('>') {
            (">", &condition[1..])
        } else if condition.starts_with('<') {
            ("<", &condition[1..])
        } else if condition.starts_with('=') {
            ("=", &condition[1..])
        } else {
            ("=", condition)
        };

        let value = value_str.parse::<i64>()
            .map_err(|_| format!("Invalid numeric value: {}", value_str))?;

        Ok(Self {
            operator: operator.to_string(),
            value,
            unit: None,
        })
    }

    /// Parse a size condition with units (GB, MB, etc.)
    pub fn parse_size(condition: &str) -> Result<Self, String> {
        let condition = condition.trim().to_uppercase();
        
        // Extract operator
        let (operator, rest) = if condition.starts_with(">=") {
            (">=", &condition[2..])
        } else if condition.starts_with("<=") {
            ("<=", &condition[2..])
        } else if condition.starts_with('>') {
            (">", &condition[1..])
        } else if condition.starts_with('<') {
            ("<", &condition[1..])
        } else {
            (">=", condition.as_str())
        };

        // Extract value and unit
        let (value_str, unit) = if rest.ends_with("GB") {
            (&rest[..rest.len()-2], Some("GB"))
        } else if rest.ends_with("MB") {
            (&rest[..rest.len()-2], Some("MB"))
        } else if rest.ends_with("KB") {
            (&rest[..rest.len()-2], Some("KB"))
        } else {
            (rest, None)
        };

        let value = value_str.parse::<i64>()
            .map_err(|_| format!("Invalid size value: {}", value_str))?;

        // Convert to bytes
        let bytes = match unit {
            Some("GB") => value * 1024 * 1024 * 1024,
            Some("MB") => value * 1024 * 1024,
            Some("KB") => value * 1024,
            _ => value, // Assume bytes
        };

        Ok(Self {
            operator: operator.to_string(),
            value: bytes,
            unit: unit.map(|s| s.to_string()),
        })
    }

    /// Evaluate the condition against a value
    pub fn evaluate(&self, test_value: i64) -> bool {
        match self.operator.as_str() {
            ">=" => test_value >= self.value,
            "<=" => test_value <= self.value,
            ">" => test_value > self.value,
            "<" => test_value < self.value,
            "=" => test_value == self.value,
            _ => false,
        }
    }
}

/// Custom format definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFormat {
    /// Unique identifier
    pub id: Uuid,
    /// Format name
    pub name: String,
    /// List of specifications (all must match unless optional)
    pub specifications: Vec<FormatSpecification>,
    /// Score boost/penalty for this format
    pub score: i32,
    /// Whether this format is enabled
    pub enabled: bool,
}

impl CustomFormat {
    /// Create a new custom format
    pub fn new(name: impl Into<String>, score: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            specifications: Vec::new(),
            score,
            enabled: true,
        }
    }

    /// Add a specification to this format
    pub fn add_spec(mut self, spec: FormatSpecification) -> Self {
        self.specifications.push(spec);
        self
    }

    /// Check if this format matches the given release data
    pub fn matches(&self, release_data: &ReleaseData) -> bool {
        if !self.enabled {
            return false;
        }

        let mut required_specs_met = true;
        let mut optional_specs_met = false;

        for spec in &self.specifications {
            let matches = spec.matches(release_data);
            
            if spec.required {
                if !matches {
                    required_specs_met = false;
                    break;
                }
            } else if matches {
                optional_specs_met = true;
            }
        }

        // Must meet all required specs and at least one optional spec
        required_specs_met && (optional_specs_met || self.specifications.iter().all(|s| s.required))
    }
}

/// Release data for custom format matching
#[derive(Debug, Clone)]
pub struct ReleaseData {
    pub title: String,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub freeleech: Option<bool>,
    pub internal: Option<bool>,
    pub indexer: String,
    pub release_group: Option<String>,
}

impl ReleaseData {
    /// Create release data from indexer result
    pub fn from_search_result(result: &crate::engine::Release) -> Self {
        Self {
            title: result.title.clone(),
            size_bytes: result.size,
            seeders: result.seeders,
            leechers: result.leechers,
            freeleech: result.freeleech,
            internal: None, // TODO: Extract from indexer data
            indexer: "Unknown".to_string(), // TODO: Pass from context
            release_group: result.release_group.clone(),
        }
    }
}

/// Custom format engine for evaluating releases
#[derive(Debug)]
pub struct CustomFormatEngine {
    /// List of available custom formats
    pub formats: Vec<CustomFormat>,
}

impl CustomFormatEngine {
    /// Create a new custom format engine
    pub fn new() -> Self {
        Self {
            formats: Self::default_formats(),
        }
    }

    /// Create engine with custom formats
    pub fn with_formats(formats: Vec<CustomFormat>) -> Self {
        Self { formats }
    }

    /// Get default custom formats
    fn default_formats() -> Vec<CustomFormat> {
        vec![
            CustomFormat::new("Freeleech", 25)
                .add_spec(FormatSpecification::new("indexer_flag", "freeleech")),
            
            CustomFormat::new("Internal", 15)
                .add_spec(FormatSpecification::new("indexer_flag", "internal")),
            
            CustomFormat::new("x265/HEVC", 5)
                .add_spec(FormatSpecification::new("codec", "x265")),
            
            CustomFormat::new("Remux", 20)
                .add_spec(FormatSpecification::new("source", "remux")),
            
            CustomFormat::new("HDR", 10)
                .add_spec(FormatSpecification::new("release_title", "HDR|Dolby.*Vision")),
            
            CustomFormat::new("Atmos", 5)
                .add_spec(FormatSpecification::new("release_title", "Atmos|DTS:X")),
            
            CustomFormat::new("Scene Release", -10)
                .add_spec(FormatSpecification::new("release_group", "scene")),
            
            CustomFormat::new("Large File", -5)
                .add_spec(FormatSpecification::new("size", ">25GB")),
            
            CustomFormat::new("High Seeders", 5)
                .add_spec(FormatSpecification::new("seeders", ">=20")),
        ]
    }

    /// Calculate custom format score for a release
    pub fn calculate_format_score(&self, release_data: &ReleaseData) -> i32 {
        let mut total_score = 0;

        for format in &self.formats {
            if format.matches(release_data) {
                total_score += format.score;
                tracing::debug!("Custom format '{}' matched, added {} points", format.name, format.score);
            }
        }

        total_score
    }

    /// Get all matching custom formats for a release
    pub fn get_matching_formats(&self, release_data: &ReleaseData) -> Vec<&CustomFormat> {
        self.formats
            .iter()
            .filter(|format| format.matches(release_data))
            .collect()
    }

    /// Add a new custom format
    pub fn add_format(&mut self, format: CustomFormat) {
        self.formats.push(format);
    }

    /// Remove a custom format by ID
    pub fn remove_format(&mut self, id: &Uuid) -> bool {
        if let Some(pos) = self.formats.iter().position(|f| f.id == *id) {
            self.formats.remove(pos);
            true
        } else {
            false
        }
    }

    /// Update an existing custom format
    pub fn update_format(&mut self, format: CustomFormat) -> bool {
        if let Some(existing) = self.formats.iter_mut().find(|f| f.id == format.id) {
            *existing = format;
            true
        } else {
            false
        }
    }

    /// Get format by ID
    pub fn get_format(&self, id: &Uuid) -> Option<&CustomFormat> {
        self.formats.iter().find(|f| f.id == *id)
    }

    /// Get all formats
    pub fn get_all_formats(&self) -> &[CustomFormat] {
        &self.formats
    }
}

impl Default for CustomFormatEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_release_data(title: &str) -> ReleaseData {
        ReleaseData {
            title: title.to_string(),
            size_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
            seeders: Some(25),
            leechers: Some(2),
            freeleech: Some(false),
            internal: Some(false),
            indexer: "TestIndexer".to_string(),
            release_group: title.split('-').last().map(|s| s.trim().to_string()),
        }
    }

    #[test]
    fn test_format_specification_title_matching() {
        let spec = FormatSpecification::new("release_title", "x265|HEVC");
        
        let release_x265 = create_test_release_data("Movie.2024.1080p.BluRay.x265-GROUP");
        let release_hevc = create_test_release_data("Movie.2024.1080p.BluRay.HEVC-GROUP");
        let release_x264 = create_test_release_data("Movie.2024.1080p.BluRay.x264-GROUP");
        
        assert!(spec.matches(&release_x265));
        assert!(spec.matches(&release_hevc));
        assert!(!spec.matches(&release_x264));
    }

    #[test]
    fn test_format_specification_freeleech_matching() {
        let spec = FormatSpecification::new("indexer_flag", "freeleech");
        
        let mut release_freeleech = create_test_release_data("Movie.2024.1080p.BluRay.x264-GROUP");
        release_freeleech.freeleech = Some(true);
        
        let release_normal = create_test_release_data("Movie.2024.1080p.BluRay.x264-GROUP");
        
        assert!(spec.matches(&release_freeleech));
        assert!(!spec.matches(&release_normal));
    }

    #[test]
    fn test_size_condition_parsing() {
        let condition = NumericCondition::parse_size(">5GB").unwrap();
        assert_eq!(condition.operator, ">");
        assert_eq!(condition.value, 5 * 1024 * 1024 * 1024);
        
        let condition2 = NumericCondition::parse_size("<=10MB").unwrap();
        assert_eq!(condition2.operator, "<=");
        assert_eq!(condition2.value, 10 * 1024 * 1024);
    }

    #[test]
    fn test_custom_format_matching() {
        let format = CustomFormat::new("x265 Format", 5)
            .add_spec(FormatSpecification::new("codec", "x265"));
        
        let release_x265 = create_test_release_data("Movie.2024.1080p.BluRay.x265-GROUP");
        let release_x264 = create_test_release_data("Movie.2024.1080p.BluRay.x264-GROUP");
        
        assert!(format.matches(&release_x265));
        assert!(!format.matches(&release_x264));
    }

    #[test]
    fn test_custom_format_engine_scoring() {
        let engine = CustomFormatEngine::new();
        
        let mut release_freeleech = create_test_release_data("Movie.2024.2160p.BluRay.x265.HDR-GROUP");
        release_freeleech.freeleech = Some(true);
        release_freeleech.seeders = Some(25);
        
        let score = engine.calculate_format_score(&release_freeleech);
        
        // Should get points for: freeleech (25) + x265 (5) + HDR (10) + high seeders (5) = 45
        assert!(score >= 40, "Score was {}, expected >= 40", score);
    }

    #[test]
    fn test_format_negation() {
        let format = CustomFormat::new("No Scene", 10)
            .add_spec(FormatSpecification::new("release_group", "scene").negate(true));
        
        let scene_release = create_test_release_data("Movie.2024.1080p.BluRay.x264-SCENE");
        let non_scene_release = create_test_release_data("Movie.2024.1080p.BluRay.x264-PRIVATE");
        
        assert!(!format.matches(&scene_release));
        assert!(format.matches(&non_scene_release));
    }
}