use super::common::{ListItem, ListParseError, ListParser, ListSource};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use regex::Regex;

/// IMDb List Parser for public lists and charts
#[derive(Debug, Clone)]
pub struct ImdbListParser {
    client: Client,
    rate_limit_delay: Duration,
}

impl ImdbListParser {
    pub fn new() -> Result<Self, ListParseError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(|e| ListParseError::Unknown(e.to_string()))?;
            
        Ok(Self {
            client,
            rate_limit_delay: Duration::from_millis(500), // 2 req/sec max
        })
    }
    
    /// Parse IMDb Top 250 chart
    pub async fn parse_top250(&self) -> Result<Vec<ListItem>, ListParseError> {
        self.parse_list("https://www.imdb.com/chart/top").await
    }
    
    /// Parse IMDb Popular Movies
    pub async fn parse_popular(&self) -> Result<Vec<ListItem>, ListParseError> {
        self.parse_list("https://www.imdb.com/chart/moviemeter").await
    }
    
    /// Parse a public IMDb list or watchlist
    async fn parse_public_list(&self, list_id: &str) -> Result<Vec<ListItem>, ListParseError> {
        let url = format!("https://www.imdb.com/list/{}/", list_id);
        self.parse_list(&url).await
    }
    
    /// Extract IMDb ID from various URL formats
    fn extract_imdb_id(&self, url: &str) -> Option<String> {
        let patterns = [
            r"/title/(tt\d+)",
            r"(tt\d+)",
        ];
        
        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(cap) = re.captures(url) {
                    return Some(cap[1].to_string());
                }
            }
        }
        None
    }
    
    /// Parse HTML response from IMDb
    async fn parse_html_response(&self, html: &str, source_url: &str) -> Result<Vec<ListItem>, ListParseError> {
        let document = Html::parse_document(html);
        let mut items = Vec::new();
        
        // Try different selectors for different page types
        let selectors = [
            // Chart pages (Top 250, Popular)
            ("td.titleColumn", "td.titleColumn a", "td.titleColumn span.secondaryInfo", "td.ratingColumn.imdbRating"),
            // List pages
            (".lister-item", ".lister-item-header a", ".lister-item-year", ".ipl-rating-star__rating"),
            // New list format
            (".ipc-metadata-list-summary-item", ".ipc-metadata-list-summary-item__t", "", ".ipc-rating-star--rating"),
        ];
        
        for (container_sel, title_sel, year_sel, rating_sel) in &selectors {
            let container = Selector::parse(container_sel).unwrap();
            
            if document.select(&container).count() > 0 {
                info!("Found {} items using selector: {}", document.select(&container).count(), container_sel);
                
                for element in document.select(&container) {
                    let title_selector = Selector::parse(title_sel).unwrap();
                    let title_elem = element.select(&title_selector).next();
                    
                    if let Some(title_elem) = title_elem {
                        let title = title_elem.text().collect::<String>().trim().to_string();
                        let href = title_elem.value().attr("href").unwrap_or("");
                        let imdb_id = self.extract_imdb_id(href);
                        
                        // Extract year if selector provided
                        let year = if !year_sel.is_empty() {
                            let year_selector = Selector::parse(year_sel).unwrap();
                            element.select(&year_selector)
                                .next()
                                .and_then(|e| {
                                    let text = e.text().collect::<String>();
                                    // Extract year from parentheses like (2024) or just the number
                                    let year_re = Regex::new(r"(\d{4})").ok()?;
                                    year_re.captures(&text)
                                        .and_then(|cap| cap[1].parse::<i32>().ok())
                                })
                        } else {
                            None
                        };
                        
                        // Extract rating if selector provided
                        let rating = if !rating_sel.is_empty() {
                            let rating_selector = Selector::parse(rating_sel).unwrap();
                            element.select(&rating_selector)
                                .next()
                                .and_then(|e| {
                                    e.text().collect::<String>()
                                        .trim()
                                        .parse::<f32>()
                                        .ok()
                                })
                        } else {
                            None
                        };
                        
                        items.push(ListItem {
                            tmdb_id: None, // Will need to be resolved later
                            imdb_id: imdb_id.clone(),
                            title: title.clone(),
                            year,
                            overview: None,
                            poster_path: None,
                            backdrop_path: None,
                            release_date: None,
                            runtime: None,
                            genres: vec![],
                            original_language: None,
                            vote_average: rating,
                            vote_count: None,
                            popularity: None,
                            source_metadata: serde_json::json!({
                                "source": "imdb",
                                "url": source_url,
                                "imdb_id": imdb_id,
                            }),
                        });
                    }
                }
                
                if !items.is_empty() {
                    break; // Found items with this selector, stop trying others
                }
            }
        }
        
        if items.is_empty() {
            warn!("No items found in IMDb list at {}", source_url);
            return Err(ListParseError::ParseError("No items found in list".to_string()));
        }
        
        info!("Parsed {} items from IMDb list", items.len());
        Ok(items)
    }
    
    /// Handle pagination for large lists
    async fn parse_paginated_list(&self, base_url: &str) -> Result<Vec<ListItem>, ListParseError> {
        let mut all_items = Vec::new();
        let mut page = 1;
        let max_pages = 10; // Limit to prevent infinite loops
        
        while page <= max_pages {
            let url = if base_url.contains('?') {
                format!("{}&page={}", base_url, page)
            } else {
                format!("{}?page={}", base_url, page)
            };
            
            debug!("Fetching page {} from {}", page, url);
            
            match self.fetch_and_parse(&url).await {
                Ok(items) => {
                    if items.is_empty() {
                        break; // No more items
                    }
                    all_items.extend(items);
                    page += 1;
                    
                    // Rate limiting
                    sleep(self.rate_limit_delay).await;
                }
                Err(e) => {
                    if page == 1 {
                        return Err(e); // First page failed, return error
                    } else {
                        break; // Subsequent page failed, return what we have
                    }
                }
            }
        }
        
        Ok(all_items)
    }
    
    /// Fetch and parse a single page
    async fn fetch_and_parse(&self, url: &str) -> Result<Vec<ListItem>, ListParseError> {
        let response = self.client
            .get(url)
            .send()
            .await?;
            
        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Err(ListParseError::NotFound);
            }
            return Err(ListParseError::HttpError(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }
        
        let html = response.text().await?;
        self.parse_html_response(&html, url).await
    }
    
    /// Export list to CSV format (IMDb compatible)
    pub fn export_csv(&self, items: &[ListItem]) -> String {
        let mut csv = String::from("Position,Const,Created,Modified,Description,Title,URL,Title Type,IMDb Rating,Runtime (mins),Year,Genres,Num Votes,Release Date,Directors\n");
        
        for (i, item) in items.iter().enumerate() {
            let position = i + 1;
            let empty_string = String::new();
            let const_id = item.imdb_id.as_ref().unwrap_or(&empty_string);
            let title = &item.title;
            let url = format!("https://www.imdb.com/title/{}/", const_id);
            let rating = item.vote_average.map(|r| r.to_string()).unwrap_or_default();
            let runtime = item.runtime.map(|r| r.to_string()).unwrap_or_default();
            let year = item.year.map(|y| y.to_string()).unwrap_or_default();
            let genres = item.genres.join(", ");
            let votes = item.vote_count.map(|v| v.to_string()).unwrap_or_default();
            let release_empty = String::new();
            let release = item.release_date.as_ref().unwrap_or(&release_empty);
            
            csv.push_str(&format!(
                "{},{},,,\"{}\",\"{}\",{},movie,{},{},{},\"{}\",{},{},\n",
                position, const_id, title, title, url, rating, runtime, year, genres, votes, release
            ));
        }
        
        csv
    }
}

#[async_trait]
impl ListParser for ImdbListParser {
    async fn parse_list(&self, list_url: &str) -> Result<Vec<ListItem>, ListParseError> {
        if !self.validate_url(list_url) {
            return Err(ListParseError::InvalidUrl(format!("Invalid IMDb URL: {}", list_url)));
        }
        
        info!("Parsing IMDb list: {}", list_url);
        
        // Check if it's a paginated list
        if list_url.contains("/list/") || list_url.contains("/user/") {
            self.parse_paginated_list(list_url).await
        } else {
            self.fetch_and_parse(list_url).await
        }
    }
    
    fn source_type(&self) -> ListSource {
        ListSource::IMDb
    }
    
    fn validate_url(&self, url: &str) -> bool {
        url.starts_with("https://www.imdb.com/") || url.starts_with("http://www.imdb.com/")
    }
}

impl Default for ImdbListParser {
    fn default() -> Self {
        Self::new().expect("Failed to create IMDb parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_imdb_id() {
        let parser = ImdbListParser::default();
        
        assert_eq!(
            parser.extract_imdb_id("/title/tt0111161/"),
            Some("tt0111161".to_string())
        );
        
        assert_eq!(
            parser.extract_imdb_id("https://www.imdb.com/title/tt0068646/?ref_=chart"),
            Some("tt0068646".to_string())
        );
        
        assert_eq!(
            parser.extract_imdb_id("tt0468569"),
            Some("tt0468569".to_string())
        );
    }
    
    #[test]
    fn test_validate_url() {
        let parser = ImdbListParser::default();
        
        assert!(parser.validate_url("https://www.imdb.com/chart/top"));
        assert!(parser.validate_url("http://www.imdb.com/list/ls000000001/"));
        assert!(!parser.validate_url("https://example.com/list"));
        assert!(!parser.validate_url("not-a-url"));
    }
}