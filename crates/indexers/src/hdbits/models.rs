//! HDBits API models and data structures
//!
//! This module contains models that match the actual HDBits API response format.
//! Key changes made:
//! - `id` field is now u32 (not string)
//! - Added missing fields: hash, descr, utadded, numfiles, filename, etc.
//! - `freeleech` is now string ("yes"/"no") instead of optional
//! - `type_*` fields are now u32 instead of strings
//! - Improved date parsing to handle "+0000" timezone format
//! - Added comprehensive deserialization tests with real API response format

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// HDBits API search request
#[derive(Debug, Clone, Serialize)]
pub struct HDBitsSearchRequest {
    pub username: String,
    pub passkey: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imdb: Option<HDBitsImdbSearch>,
}

/// IMDB search parameters for HDBits
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HDBitsImdbSearch {
    pub id: u32,
}

/// HDBits API response
#[derive(Debug, Deserialize)]
pub struct HDBitsResponse {
    pub status: u32,
    pub message: Option<String>,
    pub data: Option<Vec<HDBitsTorrent>>,
}

/// HDBits torrent information
#[derive(Debug, Clone, Deserialize)]
pub struct HDBitsTorrent {
    pub id: u32,
    pub hash: String,
    pub name: String,
    pub times_completed: u32,
    pub seeders: u32,
    pub leechers: u32,
    pub size: u64,
    pub added: String, // ISO date string with timezone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utadded: Option<u64>, // Unix timestamp
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_string_lenient"
    )]
    pub descr: Option<String>, // Description - can be very long BBCode/HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numfiles: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub type_category: u32,
    pub type_codec: u32,
    pub type_medium: u32,
    pub type_origin: u32,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_u32_or_empty_object"
    )]
    pub type_exclusive: Option<u32>,
    pub freeleech: String, // "yes" or "no"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmarked: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wishlisted: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<u32>, // Owner ID - missing from original struct
    // Optional fields that may not always be present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imdb: Option<HDBitsImdbInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tvdb: Option<u32>,
}

/// HDBits IMDB information
#[derive(Debug, Clone, Deserialize)]
pub struct HDBitsImdbInfo {
    pub id: u32,
    pub rating: Option<f32>,
    pub votes: Option<u32>,
    // Additional fields that HDBits API returns
    #[serde(skip_serializing_if = "Option::is_none")]
    pub englishtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originaltitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genres: Option<Vec<String>>,
}

/// HDBits category information
#[derive(Debug, Clone, Deserialize)]
pub struct HDBitsCategory {
    pub id: u32,
    pub name: String,
}

/// Movie search request parameters
#[derive(Debug, Clone)]
pub struct MovieSearchRequest {
    pub title: Option<String>,
    pub year: Option<u32>,
    pub imdb_id: Option<String>,
    pub limit: Option<u32>,
    pub min_seeders: Option<u32>,
}

impl MovieSearchRequest {
    pub fn new() -> Self {
        Self {
            title: None,
            year: None,
            imdb_id: None,
            limit: Some(50),
            min_seeders: Some(1),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_year(mut self, year: u32) -> Self {
        self.year = Some(year);
        self
    }

    pub fn with_imdb_id(mut self, imdb_id: &str) -> Self {
        // Extract numeric IMDB ID if needed
        let clean_id = imdb_id.trim_start_matches("tt");
        self.imdb_id = Some(clean_id.to_string());
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_min_seeders(mut self, min_seeders: u32) -> Self {
        self.min_seeders = Some(min_seeders);
        self
    }
}

impl Default for MovieSearchRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl HDBitsTorrent {
    /// Get the download URL for this torrent
    /// Note: HDBits download requires either passkey in URL or session cookie in headers
    /// For production indexer, we use passkey authentication
    pub fn download_url(&self, passkey: &str) -> String {
        // Using passkey authentication for automated downloads
        format!("https://hdbits.org/download.php/{}/{}", passkey, self.id)
    }

    /// Get the info URL for this torrent
    pub fn info_url(&self) -> String {
        format!("https://hdbits.org/details.php?id={}", self.id)
    }

    /// Parse the added date
    pub fn parsed_date(&self) -> Option<DateTime<Utc>> {
        // Try to parse HDBits format with +0000 timezone: "2025-08-09T09:34:25+0000"
        if let Ok(dt) = DateTime::parse_from_str(&self.added, "%Y-%m-%dT%H:%M:%S%z") {
            return Some(dt.with_timezone(&Utc));
        }

        // Try RFC3339 format
        if let Ok(dt) = DateTime::parse_from_rfc3339(&self.added) {
            return Some(dt.with_timezone(&Utc));
        }

        // Fallback to simple format without timezone: "2024-01-15 12:30:45"
        NaiveDateTime::parse_from_str(&self.added, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc())
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> i64 {
        self.size as i64
    }

    /// Extract scene group from release name
    pub fn scene_group(&self) -> Option<String> {
        // Common patterns for scene group extraction
        let patterns = [
            r"-([A-Z0-9][A-Za-z0-9_.-]+)$", // Standard suffix: Movie.Name.2024-GROUP
            r"\[([A-Z0-9][A-Za-z0-9_.-]+)\]", // Bracketed: [GROUP]
            r"\{([A-Z0-9][A-Za-z0-9_.-]+)\}", // Braced: {GROUP}
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(&self.name) {
                    if let Some(group) = captures.get(1) {
                        let group_name = group.as_str().to_string();
                        // Skip common non-group suffixes
                        if !is_non_group_suffix(&group_name) {
                            return Some(group_name);
                        }
                    }
                }
            }
        }

        None
    }

    /// Get IMDB ID as string
    pub fn imdb_id(&self) -> Option<String> {
        self.imdb.as_ref().map(|imdb| format!("tt{:07}", imdb.id))
    }

    /// Check if this is a freeleech torrent
    pub fn is_freeleech(&self) -> bool {
        self.freeleech == "yes"
    }

    /// Check if this is an internal release
    pub fn is_internal(&self) -> bool {
        self.type_origin == origins::INTERNAL
    }

    /// Calculate age in hours from added date
    pub fn age_hours(&self) -> Option<i32> {
        self.parsed_date().map(|added| {
            let now = Utc::now();
            let duration = now.signed_duration_since(added);
            duration.num_hours() as i32
        })
    }
}

/// Check if a suffix is likely not a scene group
fn is_non_group_suffix(suffix: &str) -> bool {
    let non_groups = [
        "PROPER",
        "REPACK",
        "INTERNAL",
        "LIMITED",
        "UNRATED",
        "DC",
        "EXTENDED",
        "THEATRICAL",
        "DIRECTORS",
        "CUT",
        "COMPLETE",
        "READNFO",
        "READ",
        "NFO",
        "x264",
        "x265",
        "H264",
        "H265",
        "HEVC",
        "AVC",
        "XVID",
        "DIVX",
        "1080p",
        "720p",
        "480p",
        "2160p",
        "4K",
        "UHD",
        "HDR",
        "SDR",
        "BLURAY",
        "BDRIP",
        "DVDRIP",
        "WEBRIP",
        "WEBDL",
        "HDTV",
        "PDTV",
        "AC3",
        "DTS",
        "AAC",
        "MP3",
        "FLAC",
        "DD5",
        "DD+",
        "ATMOS",
        "TRUEHD",
        "MULTI",
        "DUAL",
        "ENG",
        "FRENCH",
        "GERMAN",
        "SPANISH",
        "ITALIAN",
        "SUBBED",
        "DUBBED",
        "SUBS",
        "NOSUBS",
    ];

    let upper_suffix = suffix.to_uppercase();
    non_groups
        .iter()
        .any(|&non_group| upper_suffix.contains(non_group))
}

/// HDBits category constants
pub mod categories {
    /// Movie categories
    pub const MOVIE: u32 = 1;
    pub const MOVIE_BD: u32 = 2;
    pub const MOVIE_UHD: u32 = 7;

    /// All movie categories
    pub const ALL_MOVIES: &[u32] = &[MOVIE, MOVIE_BD, MOVIE_UHD];
}

/// HDBits codec constants  
pub mod codecs {
    pub const H264: &str = "H.264";
    pub const H265: &str = "H.265";
    pub const HEVC: &str = "HEVC";
    pub const X264: &str = "x264";
    pub const X265: &str = "x265";
    pub const XVID: &str = "XviD";
}

/// HDBits medium constants
pub mod mediums {
    pub const BLURAY: &str = "Blu-ray";
    pub const WEBDL: &str = "WEB-DL";
    pub const WEBRIP: &str = "WEBRip";
    pub const HDTV: &str = "HDTV";
    pub const DVD: &str = "DVD";
}

/// HDBits origin constants
pub mod origins {
    pub const INTERNAL: u32 = 1;
    pub const SCENE: u32 = 0;
}

/// Custom deserializer for optional strings that handles edge cases
/// This is lenient and will return None if deserialization fails
fn deserialize_optional_string_lenient<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // Try to deserialize as Option<String> first
    match Option::<String>::deserialize(deserializer) {
        Ok(value) => Ok(value),
        Err(_) => {
            // If that fails, return None instead of propagating the error
            // This handles cases where the field is malformed or too large
            Ok(None)
        }
    }
}

/// Custom deserializer for u32 fields that may return empty objects
/// Handles cases where HDBits API returns {} instead of null or 0
fn deserialize_u32_or_empty_object<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct U32OrEmptyObjectVisitor;

    impl<'de> Visitor<'de> for U32OrEmptyObjectVisitor {
        type Value = Option<u32>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("u32, null, or empty object")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value <= u32::MAX as u64 {
                Ok(Some(value as u32))
            } else {
                Err(E::custom(format!("u32 out of range: {}", value)))
            }
        }

        fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value >= 0 {
                Ok(Some(value as u32))
            } else {
                Err(E::custom(format!("negative value for u32: {}", value)))
            }
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value >= 0 && value <= u32::MAX as i64 {
                Ok(Some(value as u32))
            } else {
                Err(E::custom(format!("u32 out of range: {}", value)))
            }
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            // For empty object {}, just consume it and return None
            while map.next_entry::<String, serde_json::Value>()?.is_some() {
                // Consume all entries (should be none for empty object)
            }
            Ok(None)
        }
    }

    deserializer.deserialize_any(U32OrEmptyObjectVisitor)
}
