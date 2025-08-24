//! Release name parser for extracting quality information

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Parse quality information from release name
pub fn parse_quality(name: &str) -> Value {
    let mut quality = HashMap::new();

    // Resolution parsing
    if let Some(resolution) = extract_resolution(name) {
        quality.insert("resolution".to_string(), json!(resolution));
    }

    // Source parsing
    if let Some(source) = extract_source(name) {
        quality.insert("source".to_string(), json!(source));
    }

    // Codec parsing
    if let Some(codec) = extract_codec(name) {
        quality.insert("codec".to_string(), json!(codec));
    }

    // Audio parsing
    if let Some(audio) = extract_audio(name) {
        quality.insert("audio".to_string(), json!(audio));
    }

    // HDR parsing
    if let Some(hdr) = extract_hdr(name) {
        quality.insert("hdr".to_string(), json!(hdr));
    }

    // Edition parsing
    let editions = extract_editions(name);
    if !editions.is_empty() {
        quality.insert("editions".to_string(), json!(editions));
    }

    // Language parsing
    if let Some(language) = extract_language(name) {
        quality.insert("language".to_string(), json!(language));
    }

    // Calculate overall quality score
    let score = calculate_quality_score(&quality);
    quality.insert("score".to_string(), json!(score));

    json!(quality)
}

/// Extract resolution from release name
fn extract_resolution(name: &str) -> Option<String> {
    static RESOLUTION_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(2160p|1080p|720p|576p|480p|4K|UHD)\b").unwrap());

    RESOLUTION_REGEX
        .find(name)
        .map(|m| normalize_resolution(m.as_str()))
}

/// Normalize resolution strings
fn normalize_resolution(resolution: &str) -> String {
    match resolution.to_uppercase().as_str() {
        "2160P" | "4K" | "UHD" => "2160p".to_string(),
        "1080P" => "1080p".to_string(),
        "720P" => "720p".to_string(),
        "576P" => "576p".to_string(),
        "480P" => "480p".to_string(),
        _ => resolution.to_lowercase(),
    }
}

/// Extract source from release name
fn extract_source(name: &str) -> Option<String> {
    static SOURCE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)\b(BluRay|Blu-Ray|BDRip|WEBDL|WEB-DL|WEBRip|HDTV|PDTV|DVDRip|DVD|HDDVD|HD-DVD)\b",
        )
        .unwrap()
    });

    SOURCE_REGEX
        .find(name)
        .map(|m| normalize_source(m.as_str()))
}

/// Normalize source strings
fn normalize_source(source: &str) -> String {
    match source.to_uppercase().as_str() {
        "BLURAY" | "BLU-RAY" => "BluRay".to_string(),
        "BDRIP" => "BDRip".to_string(),
        "WEBDL" | "WEB-DL" => "WEB-DL".to_string(),
        "WEBRIP" => "WEBRip".to_string(),
        "HDTV" => "HDTV".to_string(),
        "PDTV" => "PDTV".to_string(),
        "DVDRIP" => "DVDRip".to_string(),
        "DVD" => "DVD".to_string(),
        "HDDVD" | "HD-DVD" => "HD-DVD".to_string(),
        _ => source.to_string(),
    }
}

/// Extract codec from release name
fn extract_codec(name: &str) -> Option<String> {
    static CODEC_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(x264|x265|H\.?264|H\.?265|HEVC|AVC|XviD|DivX|VC-1)\b").unwrap()
    });

    CODEC_REGEX.find(name).map(|m| normalize_codec(m.as_str()))
}

/// Normalize codec strings
fn normalize_codec(codec: &str) -> String {
    match codec.to_uppercase().replace(".", "").as_str() {
        "X264" => "x264".to_string(),
        "X265" | "H265" | "HEVC" => "x265".to_string(),
        "H264" | "AVC" => "x264".to_string(),
        "XVID" => "XviD".to_string(),
        "DIVX" => "DivX".to_string(),
        "VC-1" | "VC1" => "VC-1".to_string(),
        _ => codec.to_string(),
    }
}

/// Extract audio information from release name
fn extract_audio(name: &str) -> Option<String> {
    static AUDIO_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(DTS-HD\.?MA|DTS-HD|TRUEHD|TrueHD|DD\+?5\.1|DD\+?7\.1|AC3|DTS|AAC|MP3|FLAC|ATMOS|DTS-X)\b").unwrap()
    });

    AUDIO_REGEX.find(name).map(|m| normalize_audio(m.as_str()))
}

/// Normalize audio strings
fn normalize_audio(audio: &str) -> String {
    let upper = audio.to_uppercase();
    match upper.as_str() {
        s if s.starts_with("DTS-HD.MA") || s.starts_with("DTS-HDMA") => "DTS-HD MA".to_string(),
        s if s.starts_with("DTS-HD") => "DTS-HD".to_string(),
        "TRUEHD" => "TrueHD".to_string(),
        s if s.starts_with("DD+") => s.replace("DD+", "DD+"),
        s if s.starts_with("DD") => s.to_string(),
        "AC3" => "AC3".to_string(),
        "DTS" => "DTS".to_string(),
        "AAC" => "AAC".to_string(),
        "MP3" => "MP3".to_string(),
        "FLAC" => "FLAC".to_string(),
        "ATMOS" => "Atmos".to_string(),
        "DTS-X" => "DTS-X".to_string(),
        _ => audio.to_string(),
    }
}

/// Extract HDR information from release name
fn extract_hdr(name: &str) -> Option<String> {
    static HDR_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(HDR10\+?|HDR|Dolby\.?Vision|DV|SDR)\b").unwrap());

    HDR_REGEX.find(name).map(|m| normalize_hdr(m.as_str()))
}

/// Normalize HDR strings
fn normalize_hdr(hdr: &str) -> String {
    match hdr.to_uppercase().replace(".", "").as_str() {
        "HDR10+" => "HDR10+".to_string(),
        "HDR10" | "HDR" => "HDR10".to_string(),
        "DOLBYVISION" | "DV" => "Dolby Vision".to_string(),
        "SDR" => "SDR".to_string(),
        _ => hdr.to_string(),
    }
}

/// Extract edition information from release name
fn extract_editions(name: &str) -> Vec<String> {
    static EDITION_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(Director'?s?\.?Cut|Extended\.?Cut|Theatrical\.?Cut|Unrated|IMAX|Remastered|Anniversary|Special\.?Edition|Ultimate\.?Edition|Final\.?Cut|Complete)\b").unwrap()
    });

    EDITION_REGEX
        .find_iter(name)
        .map(|m| normalize_edition(m.as_str()))
        .collect()
}

/// Normalize edition strings
fn normalize_edition(edition: &str) -> String {
    let clean = edition.replace(".", " ").replace("'", "'");
    match clean.to_uppercase().as_str() {
        s if s.contains("DIRECTORS") || s.contains("DIRECTOR'S") => "Director's Cut".to_string(),
        s if s.contains("EXTENDED") => "Extended Cut".to_string(),
        s if s.contains("THEATRICAL") => "Theatrical Cut".to_string(),
        "UNRATED" => "Unrated".to_string(),
        "IMAX" => "IMAX".to_string(),
        "REMASTERED" => "Remastered".to_string(),
        "ANNIVERSARY" => "Anniversary".to_string(),
        s if s.contains("SPECIAL") => "Special Edition".to_string(),
        s if s.contains("ULTIMATE") => "Ultimate Edition".to_string(),
        s if s.contains("FINAL") => "Final Cut".to_string(),
        "COMPLETE" => "Complete".to_string(),
        _ => edition.to_string(),
    }
}

/// Extract language information from release name
fn extract_language(name: &str) -> Option<String> {
    static LANG_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(MULTI|DUAL|ENG|ENGLISH|FRENCH|GERMAN|SPANISH|ITALIAN|JAPANESE|KOREAN|CHINESE|RUSSIAN)\b").unwrap()
    });

    LANG_REGEX
        .find(name)
        .map(|m| normalize_language(m.as_str()))
}

/// Normalize language strings
fn normalize_language(language: &str) -> String {
    match language.to_uppercase().as_str() {
        "MULTI" => "Multi".to_string(),
        "DUAL" => "Dual Audio".to_string(),
        "ENG" | "ENGLISH" => "English".to_string(),
        "FRENCH" => "French".to_string(),
        "GERMAN" => "German".to_string(),
        "SPANISH" => "Spanish".to_string(),
        "ITALIAN" => "Italian".to_string(),
        "JAPANESE" => "Japanese".to_string(),
        "KOREAN" => "Korean".to_string(),
        "CHINESE" => "Chinese".to_string(),
        "RUSSIAN" => "Russian".to_string(),
        _ => language.to_string(),
    }
}

/// Calculate quality score based on parsed components
fn calculate_quality_score(quality: &HashMap<String, Value>) -> i32 {
    let mut score = 0;

    // Resolution scoring
    if let Some(resolution) = quality.get("resolution").and_then(|v| v.as_str()) {
        score += match resolution {
            "2160p" => 100,
            "1080p" => 80,
            "720p" => 60,
            "576p" => 40,
            "480p" => 20,
            _ => 0,
        };
    }

    // Source scoring
    if let Some(source) = quality.get("source").and_then(|v| v.as_str()) {
        score += match source {
            "BluRay" => 50,
            "WEB-DL" => 45,
            "WEBRip" => 40,
            "BDRip" => 35,
            "HDTV" => 30,
            "DVDRip" => 20,
            "DVD" => 15,
            _ => 0,
        };
    }

    // Codec scoring
    if let Some(codec) = quality.get("codec").and_then(|v| v.as_str()) {
        score += match codec {
            "x265" => 20,
            "x264" => 15,
            "XviD" => 10,
            _ => 0,
        };
    }

    // Audio scoring
    if let Some(audio) = quality.get("audio").and_then(|v| v.as_str()) {
        score += match audio {
            "TrueHD" | "DTS-HD MA" => 15,
            "DTS-HD" | "Atmos" | "DTS-X" => 12,
            "DTS" => 10,
            "DD+5.1" | "DD+7.1" => 8,
            "DD5.1" | "AC3" => 5,
            "AAC" => 3,
            _ => 0,
        };
    }

    // HDR scoring
    if let Some(hdr) = quality.get("hdr").and_then(|v| v.as_str()) {
        score += match hdr {
            "Dolby Vision" => 15,
            "HDR10+" => 12,
            "HDR10" => 10,
            _ => 0,
        };
    }

    // Edition bonus
    if let Some(editions) = quality.get("editions").and_then(|v| v.as_array()) {
        for edition in editions {
            if let Some(edition_str) = edition.as_str() {
                score += match edition_str {
                    "Director's Cut" | "Extended Cut" => 5,
                    "IMAX" => 8,
                    "Remastered" => 3,
                    _ => 1,
                };
            }
        }
    }

    // Freeleech bonus - check if quality metadata indicates freeleech
    if let Some(freeleech) = quality.get("freeleech").and_then(|v| v.as_bool()) {
        if freeleech {
            score += 25; // Significant bonus for freeleech torrents
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_parsing() {
        assert_eq!(
            extract_resolution("Movie.2024.2160p.BluRay.x265-GROUP"),
            Some("2160p".to_string())
        );
        assert_eq!(
            extract_resolution("Movie.2024.1080p.WEB-DL.x264-GROUP"),
            Some("1080p".to_string())
        );
        assert_eq!(
            extract_resolution("Movie.2024.720p.HDTV.x264-GROUP"),
            Some("720p".to_string())
        );
        assert_eq!(
            extract_resolution("Movie.2024.4K.UHD.BluRay.x265-GROUP"),
            Some("2160p".to_string())
        );
    }

    #[test]
    fn test_source_parsing() {
        assert_eq!(
            extract_source("Movie.2024.1080p.BluRay.x264-GROUP"),
            Some("BluRay".to_string())
        );
        assert_eq!(
            extract_source("Movie.2024.1080p.WEB-DL.x264-GROUP"),
            Some("WEB-DL".to_string())
        );
        assert_eq!(
            extract_source("Movie.2024.720p.HDTV.x264-GROUP"),
            Some("HDTV".to_string())
        );
    }

    #[test]
    fn test_codec_parsing() {
        assert_eq!(
            extract_codec("Movie.2024.1080p.BluRay.x264-GROUP"),
            Some("x264".to_string())
        );
        assert_eq!(
            extract_codec("Movie.2024.2160p.BluRay.x265.HDR-GROUP"),
            Some("x265".to_string())
        );
        assert_eq!(
            extract_codec("Movie.2024.1080p.BluRay.H.264-GROUP"),
            Some("x264".to_string())
        );
    }

    #[test]
    fn test_quality_score() {
        let quality = parse_quality("Movie.2024.2160p.BluRay.x265.TrueHD.Atmos.HDR-GROUP");
        let score = quality["score"].as_i64().unwrap() as i32;

        // 2160p (100) + BluRay (50) + x265 (20) + TrueHD (15) + HDR10 (10) = 195+
        assert!(score > 190);
    }
}
