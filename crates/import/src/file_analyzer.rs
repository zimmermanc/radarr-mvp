//! File analyzer for parsing media file information
//! 
//! This module provides advanced analysis of detected media files,
//! extracting movie information, quality details, and release metadata.

use std::path::Path;
use radarr_core::RadarrError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use tracing::{debug, warn};

/// Analyzed file information extracted from filename and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedFile {
    /// Original file path
    pub path: std::path::PathBuf,
    /// Extracted movie title (cleaned)
    pub title: Option<String>,
    /// Release year if detected
    pub year: Option<u16>,
    /// Detected quality information
    pub quality: QualityInfo,
    /// Release group that created this file
    pub release_group: Option<String>,
    /// Whether this is a sample file
    pub is_sample: bool,
    /// Confidence score (0.0 - 1.0) for the analysis
    pub confidence: f32,
    /// Raw filename for reference
    pub original_filename: String,
}

/// Quality information extracted from filename
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityInfo {
    /// Resolution (720p, 1080p, 2160p, etc.)
    pub resolution: Option<String>,
    /// Video codec (x264, x265, h264, etc.)
    pub codec: Option<String>,
    /// Audio codec (DTS, AC3, AAC, etc.)
    pub audio: Option<String>,
    /// Source type (BluRay, WEB-DL, HDTV, etc.)
    pub source: Option<String>,
    /// HDR information (HDR, HDR10, Dolby Vision, etc.)
    pub hdr: Option<String>,
}

impl Default for QualityInfo {
    fn default() -> Self {
        Self {
            resolution: None,
            codec: None,
            audio: None,
            source: None,
            hdr: None,
        }
    }
}

/// Regular expressions for parsing movie information
static YEAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(19[0-9]{2}|20[0-4][0-9])\b").unwrap()
});

static RESOLUTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(2160p|1080p|720p|480p|4K|UHD)\b").unwrap()
});

static CODEC_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(x264|x265|h264|h265|HEVC|AVC|XviD|DivX)\b").unwrap()
});

static AUDIO_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(DTS|AC3|AAC|MP3|FLAC|TrueHD|Atmos|DD|EAC3)\b").unwrap()
});

static SOURCE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(BluRay|BDRip|WEB-DL|WEBRip|HDTV|DVDRip|CAM|TS|TC|R5|SCREENER)\b").unwrap()
});

static HDR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(HDR|HDR10|Dolby.Vision|DV|Vision)\b").unwrap()
});

static RELEASE_GROUP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-([A-Z0-9]+)(?:\[.*?\])?$").unwrap()
});

static SAMPLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(sample|trailer|preview|proof|rarbg)\b").unwrap()
});

/// Common words to remove when cleaning movie titles
const STOP_WORDS: &[&str] = &[
    "720p", "1080p", "2160p", "4K", "UHD", "BluRay", "BDRip", "WEB-DL", "WEBRip",
    "HDTV", "DVDRip", "x264", "x265", "h264", "h265", "HEVC", "DTS", "AC3", "AAC",
    "INTERNAL", "REPACK", "PROPER", "READ.NFO", "HDR", "HDR10", "Dolby", "Vision",
    "REMUX", "HYBRID", "EXTENDED", "UNRATED", "DIRECTORS", "CUT", "IMAX"
];

/// File analyzer for extracting metadata from filenames
pub struct FileAnalyzer;

impl FileAnalyzer {
    /// Create a new file analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze a file and extract metadata from its filename
    pub fn analyze_file(&self, file_path: &Path) -> Result<AnalyzedFile, RadarrError> {
        let filename = file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| RadarrError::ValidationError {
                field: "filename".to_string(),
                message: "Invalid filename encoding".to_string(),
            })?;

        debug!("Analyzing file: {}", filename);

        let original_filename = filename.to_string();
        let mut confidence: f32 = 0.0;

        // Check if it's a sample file
        let is_sample = SAMPLE_REGEX.is_match(filename);
        if is_sample {
            confidence += 0.9; // High confidence for sample detection
        }

        // Extract year
        let year = self.extract_year(filename);
        if year.is_some() {
            confidence += 0.3;
        }

        // Extract quality information
        let quality = self.extract_quality(filename);
        if quality.resolution.is_some() {
            confidence += 0.2;
        }
        if quality.source.is_some() {
            confidence += 0.2;
        }

        // Extract release group
        let release_group = self.extract_release_group(filename);
        if release_group.is_some() {
            confidence += 0.1;
        }

        // Extract and clean movie title
        let title = self.extract_title(filename, year);
        if title.is_some() {
            confidence += 0.2;
        }

        // Cap confidence at 1.0
        confidence = confidence.min(1.0);

        debug!("Analysis result - Title: {:?}, Year: {:?}, Quality: {:?}, Confidence: {:.2}", 
               title, year, quality.resolution, confidence);

        Ok(AnalyzedFile {
            path: file_path.to_path_buf(),
            title,
            year,
            quality,
            release_group,
            is_sample,
            confidence,
            original_filename,
        })
    }

    /// Extract year from filename
    fn extract_year(&self, filename: &str) -> Option<u16> {
        YEAR_REGEX
            .find(filename)
            .and_then(|m| m.as_str().parse().ok())
            .filter(|&year| year >= 1900 && year <= 2050)
    }

    /// Extract quality information from filename
    fn extract_quality(&self, filename: &str) -> QualityInfo {
        let resolution = RESOLUTION_REGEX
            .find(filename)
            .map(|m| m.as_str().to_uppercase());

        let codec = CODEC_REGEX
            .find(filename)
            .map(|m| m.as_str().to_uppercase());

        let audio = AUDIO_REGEX
            .find(filename)
            .map(|m| m.as_str().to_uppercase());

        let source = SOURCE_REGEX
            .find(filename)
            .map(|m| m.as_str().to_uppercase());

        let hdr = HDR_REGEX
            .find(filename)
            .map(|m| m.as_str().to_uppercase());

        QualityInfo {
            resolution,
            codec,
            audio,
            source,
            hdr,
        }
    }

    /// Extract release group from filename
    fn extract_release_group(&self, filename: &str) -> Option<String> {
        RELEASE_GROUP_REGEX
            .captures(filename)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Extract and clean movie title from filename
    fn extract_title(&self, filename: &str, year: Option<u16>) -> Option<String> {
        let mut title = filename.to_string();

        // Remove file extension if present
        if let Some(pos) = title.rfind('.') {
            title = title[..pos].to_string();
        }

        // Remove year if found
        if let Some(year) = year {
            title = title.replace(&year.to_string(), "");
        }

        // Remove quality indicators and other metadata
        for stop_word in STOP_WORDS {
            title = title.replace(stop_word, " ");
        }

        // Remove release group (everything after last dash)
        if let Some(pos) = title.rfind('-') {
            title = title[..pos].to_string();
        }

        // Clean up the title
        title = title
            .replace('.', " ")          // Dots to spaces
            .replace('_', " ")          // Underscores to spaces
            .replace('[', " ")          // Remove brackets
            .replace(']', " ")
            .replace('(', " ")          // Remove parentheses
            .replace(')', " ");

        // Remove extra whitespace and normalize
        title = title
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            .trim()
            .to_string();

        // Return None if title is too short or empty
        if title.len() < 2 {
            warn!("Extracted title too short: '{}'", title);
            None
        } else {
            Some(title)
        }
    }

    /// Batch analyze multiple files
    pub fn analyze_files(&self, file_paths: &[&Path]) -> Result<Vec<AnalyzedFile>, RadarrError> {
        let mut results = Vec::with_capacity(file_paths.len());
        
        for path in file_paths {
            match self.analyze_file(path) {
                Ok(analyzed) => results.push(analyzed),
                Err(e) => {
                    warn!("Failed to analyze file {}: {}", path.display(), e);
                    // Continue with other files
                }
            }
        }
        
        Ok(results)
    }

    /// Check if a filename indicates a sample file
    pub fn is_sample_file(&self, filename: &str) -> bool {
        SAMPLE_REGEX.is_match(filename)
    }

    /// Extract just the quality score for ranking purposes
    pub fn calculate_quality_score(&self, quality: &QualityInfo) -> u32 {
        let mut score = 0;

        // Resolution scoring
        if let Some(ref resolution) = quality.resolution {
            score += match resolution.as_str() {
                "2160P" | "4K" | "UHD" => 1000,
                "1080P" => 500,
                "720P" => 250,
                "480P" => 100,
                _ => 50,
            };
        }

        // Codec scoring
        if let Some(ref codec) = quality.codec {
            score += match codec.as_str() {
                "X265" | "H265" | "HEVC" => 200,
                "X264" | "H264" | "AVC" => 150,
                _ => 50,
            };
        }

        // Source scoring
        if let Some(ref source) = quality.source {
            score += match source.as_str() {
                "BLURAY" | "BDREMUX" => 300,
                "BDRIP" => 250,
                "WEB-DL" | "WEBDL" => 200,
                "WEBRIP" => 150,
                "HDTV" => 100,
                "DVDRIP" => 50,
                _ => 25,
            };
        }

        // HDR bonus
        if quality.hdr.is_some() {
            score += 100;
        }

        score
    }
}

impl Default for FileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_year() {
        let analyzer = FileAnalyzer::new();
        
        assert_eq!(analyzer.extract_year("Movie.Title.2023.1080p.BluRay.x264"), Some(2023));
        assert_eq!(analyzer.extract_year("Old.Movie.1995.DVDRip"), Some(1995));
        assert_eq!(analyzer.extract_year("No.Year.Movie.1080p"), None);
        assert_eq!(analyzer.extract_year("Invalid.Year.3000.1080p"), None);
    }

    #[test]
    fn test_extract_quality() {
        let analyzer = FileAnalyzer::new();
        
        let quality = analyzer.extract_quality("Movie.2023.1080p.BluRay.x264.DTS-GROUP");
        assert_eq!(quality.resolution, Some("1080P".to_string()));
        assert_eq!(quality.codec, Some("X264".to_string()));
        assert_eq!(quality.audio, Some("DTS".to_string()));
        assert_eq!(quality.source, Some("BLURAY".to_string()));
        
        let quality_hdr = analyzer.extract_quality("Movie.2023.2160p.WEB-DL.x265.HDR10-GROUP");
        assert_eq!(quality_hdr.resolution, Some("2160P".to_string()));
        assert_eq!(quality_hdr.hdr, Some("HDR10".to_string()));
    }

    #[test]
    fn test_extract_release_group() {
        let analyzer = FileAnalyzer::new();
        
        assert_eq!(
            analyzer.extract_release_group("Movie.2023.1080p.BluRay.x264-GROUP"),
            Some("GROUP".to_string())
        );
        
        assert_eq!(
            analyzer.extract_release_group("Movie.2023.1080p.BluRay.x264-SCENE[rartv]"),
            Some("SCENE".to_string())
        );
        
        assert_eq!(analyzer.extract_release_group("Movie.2023.1080p.BluRay.x264"), None);
    }

    #[test]
    fn test_extract_title() {
        let analyzer = FileAnalyzer::new();
        
        assert_eq!(
            analyzer.extract_title("The.Matrix.1999.1080p.BluRay.x264-GROUP", Some(1999)),
            Some("The Matrix".to_string())
        );
        
        assert_eq!(
            analyzer.extract_title("Movie_Title_2023_1080p_WEB-DL", Some(2023)),
            Some("Movie Title".to_string())
        );
        
        assert_eq!(
            analyzer.extract_title("Short.2023.1080p", Some(2023)),
            Some("Short".to_string())
        );
    }

    #[test]
    fn test_is_sample_file() {
        let analyzer = FileAnalyzer::new();
        
        assert!(analyzer.is_sample_file("Movie.2023.SAMPLE.mkv"));
        assert!(analyzer.is_sample_file("sample-movie.mp4"));
        assert!(analyzer.is_sample_file("Movie.2023.Trailer.mp4"));
        assert!(!analyzer.is_sample_file("Movie.2023.1080p.BluRay.mkv"));
    }

    #[test]
    fn test_quality_score() {
        let analyzer = FileAnalyzer::new();
        
        let high_quality = QualityInfo {
            resolution: Some("2160P".to_string()),
            codec: Some("X265".to_string()),
            source: Some("BLURAY".to_string()),
            hdr: Some("HDR10".to_string()),
            audio: None,
        };
        
        let low_quality = QualityInfo {
            resolution: Some("720P".to_string()),
            codec: Some("X264".to_string()),
            source: Some("HDTV".to_string()),
            hdr: None,
            audio: None,
        };
        
        assert!(analyzer.calculate_quality_score(&high_quality) > analyzer.calculate_quality_score(&low_quality));
    }

    #[test]
    fn test_analyze_file() {
        let analyzer = FileAnalyzer::new();
        let path = PathBuf::from("/downloads/The.Matrix.1999.1080p.BluRay.x264.DTS-GROUP.mkv");
        
        let result = analyzer.analyze_file(&path).unwrap();
        
        assert_eq!(result.title, Some("The Matrix".to_string()));
        assert_eq!(result.year, Some(1999));
        assert_eq!(result.quality.resolution, Some("1080P".to_string()));
        assert_eq!(result.release_group, Some("GROUP".to_string()));
        assert!(!result.is_sample);
        assert!(result.confidence > 0.5);
    }
}