//! RSS feed monitoring and calendar-based automation

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use uuid::Uuid;
use crate::{Result, RadarrError};

/// RSS feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeed {
    /// Unique identifier
    pub id: Uuid,
    /// Feed name
    pub name: String,
    /// Feed URL
    pub url: String,
    /// Check interval in minutes
    pub interval_minutes: u32,
    /// Whether the feed is enabled
    pub enabled: bool,
    /// Last check time
    pub last_check: Option<DateTime<Utc>>,
    /// Last successful sync
    pub last_sync: Option<DateTime<Utc>>,
    /// Categories to monitor (empty = all)
    pub categories: Vec<String>,
    /// Quality profiles to apply
    pub quality_profile_id: Option<Uuid>,
    /// Tags to apply to items
    pub tags: Vec<String>,
}

impl RssFeed {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            url: url.into(),
            interval_minutes: 15,
            enabled: true,
            last_check: None,
            last_sync: None,
            categories: Vec::new(),
            quality_profile_id: None,
            tags: Vec::new(),
        }
    }
    
    /// Check if the feed is due for checking
    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        match self.last_check {
            None => true,
            Some(last) => {
                let interval = Duration::minutes(self.interval_minutes as i64);
                Utc::now() - last >= interval
            }
        }
    }
}

/// RSS item from a feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssItem {
    /// Item GUID
    pub guid: String,
    /// Item title
    pub title: String,
    /// Item description
    pub description: Option<String>,
    /// Download URL
    pub url: String,
    /// Publication date
    pub pub_date: DateTime<Utc>,
    /// Size in bytes
    pub size: Option<u64>,
    /// Category
    pub category: Option<String>,
    /// Seeders count
    pub seeders: Option<u32>,
    /// Leechers count
    pub leechers: Option<u32>,
    /// Info hash for torrents
    pub info_hash: Option<String>,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// Calendar entry for scheduled searches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEntry {
    /// Movie ID
    pub movie_id: Uuid,
    /// Movie title
    pub title: String,
    /// Release date
    pub release_date: DateTime<Utc>,
    /// Digital release date
    pub digital_release: Option<DateTime<Utc>>,
    /// Physical release date
    pub physical_release: Option<DateTime<Utc>>,
    /// Whether to monitor for this release
    pub monitored: bool,
    /// Search offset days (search X days before/after release)
    pub search_offset_days: i32,
}

impl CalendarEntry {
    /// Check if search should be triggered
    pub fn should_search(&self) -> bool {
        if !self.monitored {
            return false;
        }
        
        let now = Utc::now();
        let offset = Duration::days(self.search_offset_days as i64);
        
        // Check each release date
        if let Some(digital) = self.digital_release {
            if now >= digital - offset {
                return true;
            }
        }
        
        if let Some(physical) = self.physical_release {
            if now >= physical - offset {
                return true;
            }
        }
        
        // Fall back to main release date
        now >= self.release_date - offset
    }
    
    /// Get the next search date
    pub fn next_search_date(&self) -> Option<DateTime<Utc>> {
        if !self.monitored {
            return None;
        }
        
        let offset = Duration::days(self.search_offset_days as i64);
        let mut dates = Vec::new();
        
        // Collect all potential search dates
        dates.push(self.release_date - offset);
        
        if let Some(digital) = self.digital_release {
            dates.push(digital - offset);
        }
        
        if let Some(physical) = self.physical_release {
            dates.push(physical - offset);
        }
        
        // Find the next future date
        let now = Utc::now();
        dates.into_iter()
            .filter(|&date| date > now)
            .min()
    }
}

/// RSS monitoring service
#[derive(Debug, Clone)]
pub struct RssMonitor {
    feeds: Vec<RssFeed>,
    calendar: Vec<CalendarEntry>,
}

impl RssMonitor {
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            calendar: Vec::new(),
        }
    }
    
    /// Add a new RSS feed
    pub fn add_feed(&mut self, feed: RssFeed) {
        self.feeds.push(feed);
    }
    
    /// Remove a feed by ID
    pub fn remove_feed(&mut self, id: Uuid) {
        self.feeds.retain(|f| f.id != id);
    }
    
    /// Get all feeds that are due for checking
    pub fn get_due_feeds(&self) -> Vec<&RssFeed> {
        self.feeds.iter()
            .filter(|f| f.is_due())
            .collect()
    }
    
    /// Mark a feed as checked
    pub fn mark_feed_checked(&mut self, id: Uuid) {
        if let Some(feed) = self.feeds.iter_mut().find(|f| f.id == id) {
            feed.last_check = Some(Utc::now());
        }
    }
    
    /// Mark a feed as successfully synced
    pub fn mark_feed_synced(&mut self, id: Uuid) {
        if let Some(feed) = self.feeds.iter_mut().find(|f| f.id == id) {
            let now = Utc::now();
            feed.last_check = Some(now);
            feed.last_sync = Some(now);
        }
    }
    
    /// Add a calendar entry
    pub fn add_calendar_entry(&mut self, entry: CalendarEntry) {
        self.calendar.push(entry);
    }
    
    /// Get calendar entries that should trigger searches
    pub fn get_searchable_entries(&self) -> Vec<&CalendarEntry> {
        self.calendar.iter()
            .filter(|e| e.should_search())
            .collect()
    }
    
    /// Get upcoming calendar entries
    pub fn get_upcoming_entries(&self, days: i64) -> Vec<&CalendarEntry> {
        let cutoff = Utc::now() + Duration::days(days);
        
        self.calendar.iter()
            .filter(|e| {
                if let Some(next) = e.next_search_date() {
                    next <= cutoff
                } else {
                    false
                }
            })
            .collect()
    }
}

impl Default for RssMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// RSS parser for feed items
pub struct RssParser;

impl RssParser {
    /// Parse RSS feed content
    pub fn parse_feed(content: &str) -> Result<Vec<RssItem>> {
        // This is a simplified parser - in production you'd use a proper RSS library
        let mut items = Vec::new();
        
        // Basic XML parsing (would use quick-xml or similar in production)
        if content.contains("<rss") || content.contains("<feed") {
            // Extract items between <item> tags
            let item_regex = regex::Regex::new(r"<item>(.*?)</item>")
                .map_err(|e| RadarrError::ValidationError {
                    field: "regex".to_string(),
                    message: e.to_string(),
                })?;
            
            for cap in item_regex.captures_iter(content) {
                if let Some(item_content) = cap.get(1) {
                    if let Ok(item) = Self::parse_item(item_content.as_str()) {
                        items.push(item);
                    }
                }
            }
        }
        
        Ok(items)
    }
    
    /// Parse a single RSS item
    fn parse_item(content: &str) -> Result<RssItem> {
        // Extract basic fields
        let title = Self::extract_tag(content, "title")
            .ok_or_else(|| RadarrError::ValidationError {
                field: "title".to_string(),
                message: "Missing title in RSS item".to_string(),
            })?;
        
        let guid = Self::extract_tag(content, "guid")
            .unwrap_or_else(|| title.clone());
        
        let url = Self::extract_tag(content, "link")
            .or_else(|| Self::extract_tag(content, "enclosure url"))
            .ok_or_else(|| RadarrError::ValidationError {
                field: "url".to_string(),
                message: "Missing URL in RSS item".to_string(),
            })?;
        
        let description = Self::extract_tag(content, "description");
        let category = Self::extract_tag(content, "category");
        
        // Parse size from enclosure
        let size = Self::extract_attribute(content, "enclosure", "length")
            .and_then(|s| s.parse::<u64>().ok());
        
        // Parse date
        let pub_date = Self::extract_tag(content, "pubDate")
            .and_then(|s| DateTime::parse_from_rfc2822(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        
        Ok(RssItem {
            guid,
            title,
            description,
            url,
            pub_date,
            size,
            category,
            seeders: None,
            leechers: None,
            info_hash: None,
            attributes: HashMap::new(),
        })
    }
    
    /// Extract tag content
    fn extract_tag(content: &str, tag: &str) -> Option<String> {
        let pattern = format!("<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
        regex::Regex::new(&pattern).ok()
            .and_then(|re| re.captures(content))
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }
    
    /// Extract attribute value
    fn extract_attribute(content: &str, tag: &str, attr: &str) -> Option<String> {
        let pattern = format!(r#"<{tag}[^>]*{attr}="([^"]+)"#, tag = regex::escape(tag), attr = regex::escape(attr));
        regex::Regex::new(&pattern).ok()
            .and_then(|re| re.captures(content))
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rss_feed_is_due() {
        let mut feed = RssFeed::new("Test Feed", "http://example.com/rss");
        
        // New feed should be due
        assert!(feed.is_due());
        
        // Recently checked feed should not be due
        feed.last_check = Some(Utc::now());
        assert!(!feed.is_due());
        
        // Old check should be due
        feed.last_check = Some(Utc::now() - Duration::hours(1));
        assert!(feed.is_due());
        
        // Disabled feed should never be due
        feed.enabled = false;
        assert!(!feed.is_due());
    }
    
    #[test]
    fn test_calendar_entry_should_search() {
        let mut entry = CalendarEntry {
            movie_id: Uuid::new_v4(),
            title: "Test Movie".to_string(),
            release_date: Utc::now() + Duration::days(7),
            digital_release: Some(Utc::now() - Duration::days(1)),
            physical_release: None,
            monitored: true,
            search_offset_days: 0,
        };
        
        // Should search because digital release is in the past
        assert!(entry.should_search());
        
        // Unmonitored should not search
        entry.monitored = false;
        assert!(!entry.should_search());
    }
    
    #[test]
    fn test_rss_parser_basic() {
        let rss_content = r#"
            <rss>
                <channel>
                    <item>
                        <title>Test Movie 2024 1080p</title>
                        <guid>12345</guid>
                        <link>http://example.com/download/12345</link>
                        <description>A test movie</description>
                        <category>Movies</category>
                        <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
                    </item>
                </channel>
            </rss>
        "#;
        
        let items = RssParser::parse_feed(rss_content).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test Movie 2024 1080p");
        assert_eq!(items[0].guid, "12345");
    }
}