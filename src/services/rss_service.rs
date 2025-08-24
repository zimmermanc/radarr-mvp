//! RSS monitoring service for automated discovery

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use radarr_core::{
    Result, RadarrError,
    rss::{RssFeed, RssItem, RssMonitor, RssParser, CalendarEntry},
    events::{EventBus, SystemEvent},
    progress::{ProgressTracker, OperationType},
    models::{Movie, QueueItem, QueuePriority},
    domain::repositories::{MovieRepository, QueueRepository},
};
use radarr_indexers::{IndexerClient, SearchRequest};
use radarr_infrastructure::DatabasePool;
use radarr_decision::{DecisionEngine, Release};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use chrono::Utc;

/// RSS monitoring service configuration
#[derive(Debug, Clone)]
pub struct RssServiceConfig {
    /// How often to check RSS feeds (seconds)
    pub check_interval_seconds: u64,
    /// How often to check calendar (seconds)
    pub calendar_interval_seconds: u64,
    /// Maximum items to process per feed
    pub max_items_per_feed: usize,
    /// Whether RSS monitoring is enabled
    pub enabled: bool,
}

impl Default for RssServiceConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 300, // 5 minutes
            calendar_interval_seconds: 3600, // 1 hour
            max_items_per_feed: 100,
            enabled: true,
        }
    }
}

/// RSS monitoring service
pub struct RssService {
    config: RssServiceConfig,
    monitor: Arc<RwLock<RssMonitor>>,
    indexer_client: Arc<dyn IndexerClient + Send + Sync>,
    database_pool: DatabasePool,
    event_bus: Option<Arc<EventBus>>,
    progress_tracker: Option<Arc<ProgressTracker>>,
    processed_items: Arc<RwLock<Vec<String>>>, // Track processed GUIDs
    movie_repository: Arc<dyn MovieRepository + Send + Sync>,
    queue_repository: Arc<dyn QueueRepository + Send + Sync>,
    decision_engine: Option<DecisionEngine>,
}

impl RssService {
    /// Create new RSS service
    pub fn new(
        config: RssServiceConfig,
        indexer_client: Arc<dyn IndexerClient + Send + Sync>,
        database_pool: DatabasePool,
        movie_repository: Arc<dyn MovieRepository + Send + Sync>,
        queue_repository: Arc<dyn QueueRepository + Send + Sync>,
    ) -> Self {
        Self {
            config,
            monitor: Arc::new(RwLock::new(RssMonitor::new())),
            indexer_client,
            database_pool,
            event_bus: None,
            progress_tracker: None,
            processed_items: Arc::new(RwLock::new(Vec::new())),
            movie_repository,
            queue_repository,
            decision_engine: None,
        }
    }
    
    /// Set event bus
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }
    
    /// Set progress tracker
    pub fn with_progress_tracker(mut self, tracker: Arc<ProgressTracker>) -> Self {
        self.progress_tracker = Some(tracker);
        self
    }
    
    /// Set decision engine for quality evaluation
    pub fn with_decision_engine(mut self, engine: DecisionEngine) -> Self {
        self.decision_engine = Some(engine);
        self
    }
    
    /// Start the RSS monitoring service
    pub async fn start(self: Arc<Self>) -> Result<()> {
        if !self.config.enabled {
            info!("RSS monitoring is disabled");
            return Ok(());
        }
        
        info!("Starting RSS monitoring service");
        
        // Spawn RSS feed checker
        let rss_service = self.clone();
        tokio::spawn(async move {
            rss_service.run_rss_monitor().await;
        });
        
        // Spawn calendar checker
        let calendar_service = self.clone();
        tokio::spawn(async move {
            calendar_service.run_calendar_monitor().await;
        });
        
        Ok(())
    }
    
    /// Run RSS feed monitoring loop
    async fn run_rss_monitor(&self) {
        let mut check_interval = interval(Duration::from_secs(self.config.check_interval_seconds));
        
        loop {
            check_interval.tick().await;
            
            let due_feeds = {
                let monitor = self.monitor.read().await;
                monitor.get_due_feeds().into_iter().cloned().collect::<Vec<_>>()
            };
            
            for feed in due_feeds {
                let feed_id = feed.id;
                let feed_name = feed.name.clone();
                
                debug!("Checking RSS feed: {}", feed_name);
                
                // Start progress tracking
                let progress_id = if let Some(tracker) = &self.progress_tracker {
                    Some(tracker.start_operation(
                        OperationType::IndexerSearch,
                        format!("Checking RSS: {}", feed_name)
                    ).await)
                } else {
                    None
                };
                
                // Check the feed
                match self.check_feed(&feed).await {
                    Ok(new_items) => {
                        info!("Found {} new items in feed {}", new_items, feed_name);
                        
                        // Complete progress
                        if let (Some(tracker), Some(id)) = (&self.progress_tracker, progress_id) {
                            tracker.complete_operation(
                                id,
                                format!("Processed {} new items", new_items)
                            ).await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to check RSS feed {}: {}", feed_name, e);
                        
                        // Fail progress
                        if let (Some(tracker), Some(id)) = (&self.progress_tracker, progress_id) {
                            tracker.fail_operation(id, e.to_string()).await;
                        }
                    }
                }
                
                // Mark feed as checked
                {
                    let mut monitor = self.monitor.write().await;
                    monitor.mark_feed_checked(feed_id);
                }
            }
        }
    }
    
    /// Check a single RSS feed
    async fn check_feed(&self, feed: &RssFeed) -> Result<usize> {
        // Fetch RSS content
        let client = reqwest::Client::new();
        let response = client.get(&feed.url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| RadarrError::NetworkError {
                message: format!("Failed to fetch RSS feed: {}", e),
            })?;
        
        let content = response.text().await
            .map_err(|e| RadarrError::NetworkError {
                message: format!("Failed to read RSS content: {}", e),
            })?;
        
        // Parse RSS items
        let items = RssParser::parse_feed(&content)?;
        
        // Filter and process new items
        let mut processed = self.processed_items.write().await;
        let mut new_count = 0;
        
        for item in items.iter().take(self.config.max_items_per_feed) {
            // Skip if already processed
            if processed.contains(&item.guid) {
                continue;
            }
            
            // Process the item
            if self.should_process_item(item, feed).await {
                self.process_rss_item(item, feed).await?;
                new_count += 1;
            }
            
            // Mark as processed
            processed.push(item.guid.clone());
            
            // Keep processed list from growing too large
            if processed.len() > 10000 {
                processed.drain(0..5000);
            }
        }
        
        Ok(new_count)
    }
    
    /// Find a matching movie for a release title
    async fn find_matching_movie(&self, release_title: &str) -> Result<Option<Movie>> {
        // Extract movie title from release name (remove quality, year, etc.)
        let cleaned_title = self.extract_movie_title(release_title);
        
        // Search for movies with similar titles
        match self.movie_repository.search_by_title(&cleaned_title, 10).await {
            Ok(movies) => {
                // Find best match based on title similarity
                for movie in movies {
                    if self.titles_match(&movie.title, &cleaned_title) {
                        return Ok(Some(movie));
                    }
                }
                Ok(None)
            }
            Err(e) => {
                error!("Failed to search movies by title '{}': {}", cleaned_title, e);
                Err(e)
            }
        }
    }
    
    /// Extract movie title from release name
    fn extract_movie_title(&self, release_name: &str) -> String {
        // Basic title extraction - remove common patterns
        let title = release_name
            // Remove file extensions
            .replace(".mkv", "")
            .replace(".mp4", "")
            .replace(".avi", "")
            // Replace dots and underscores with spaces
            .replace('.', " ")
            .replace('_', " ")
            // Remove year patterns (4 digits)
            .split_whitespace()
            .take_while(|word| !word.chars().all(|c| c.is_ascii_digit() && word.len() == 4))
            .collect::<Vec<_>>()
            .join(" ")
            // Remove quality indicators
            .split_whitespace()
            .take_while(|word| {
                let word_lower = word.to_lowercase();
                !word_lower.contains("1080p") &&
                !word_lower.contains("720p") &&
                !word_lower.contains("2160p") &&
                !word_lower.contains("4k") &&
                !word_lower.contains("bluray") &&
                !word_lower.contains("web-dl") &&
                !word_lower.contains("hdtv")
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
            
        if title.is_empty() {
            release_name.to_string()
        } else {
            title
        }
    }
    
    /// Check if two movie titles match (case-insensitive, flexible matching)
    fn titles_match(&self, movie_title: &str, extracted_title: &str) -> bool {
        let movie_lower = movie_title.to_lowercase();
        let extracted_lower = extracted_title.to_lowercase();
        
        // Exact match
        if movie_lower == extracted_lower {
            return true;
        }
        
        // Check if one contains the other
        if movie_lower.contains(&extracted_lower) || extracted_lower.contains(&movie_lower) {
            return true;
        }
        
        // Check word-by-word matching (allow some flexibility)
        let movie_words: Vec<&str> = movie_lower.split_whitespace().collect();
        let extracted_words: Vec<&str> = extracted_lower.split_whitespace().collect();
        
        // If most words match, consider it a match
        let matching_words = movie_words.iter()
            .filter(|word| extracted_words.contains(word))
            .count();
            
        let total_words = movie_words.len().min(extracted_words.len());
        if total_words > 0 && matching_words as f32 / total_words as f32 > 0.7 {
            return true;
        }
        
        false
    }
    
    /// Check if an RSS item should be processed
    async fn should_process_item(&self, item: &RssItem, feed: &RssFeed) -> bool {
        // Check category filter
        if !feed.categories.is_empty() {
            if let Some(category) = &item.category {
                if !feed.categories.contains(category) {
                    return false;
                }
            }
        }
        
        // Check age (don't process very old items)
        let age = Utc::now() - item.pub_date;
        if age.num_days() > 7 {
            return false;
        }
        
        // Check if we have a decision engine for quality evaluation
        let decision_engine = match &self.decision_engine {
            Some(engine) => engine,
            None => {
                warn!("No decision engine configured, accepting all items");
                return true;
            }
        };
        
        // Parse release information from the title
        let release = Release::from_title(item.title.clone(), item.url.clone());
        
        // Check if the release meets quality requirements
        if decision_engine.evaluate_release(&release).is_none() {
            debug!("RSS item '{}' doesn't meet quality requirements", item.title);
            return false;
        }
        
        // Check if movie exists and is monitored by searching monitored movies
        // This is a basic implementation - in production you'd want more sophisticated matching
        match self.find_matching_movie(&item.title).await {
            Ok(Some(movie)) => {
                if movie.monitored {
                    debug!("Found monitored movie '{}' for RSS item '{}'", movie.title, item.title);
                    true
                } else {
                    debug!("Found movie '{}' but it's not monitored", movie.title);
                    false
                }
            }
            Ok(None) => {
                debug!("No matching movie found for RSS item '{}'", item.title);
                false
            }
            Err(e) => {
                error!("Error searching for movie: {}", e);
                false
            }
        }
    }
    
    /// Process an RSS item
    async fn process_rss_item(&self, item: &RssItem, feed: &RssFeed) -> Result<()> {
        info!("Processing RSS item: {}", item.title);
        
        // Parse release name to create a Release object
        let mut release = Release::from_title(item.title.clone(), item.url.clone());
        
        // Set additional release information if available
        if let Some(size) = item.size {
            release = release.with_size(size);
        }
        if let Some(seeders) = item.seeders {
            release = release.with_seeders(seeders);
        }
        if let Some(leechers) = item.leechers {
            release = release.with_leechers(leechers);
        }
        
        // Calculate age in hours
        let age = Utc::now() - item.pub_date;
        let age_hours = age.num_hours().max(0) as u32;
        release = release.with_age_hours(age_hours);
        
        // Find the matching movie
        let movie = match self.find_matching_movie(&item.title).await? {
            Some(movie) => movie,
            None => {
                warn!("No matching movie found for RSS item: {}", item.title);
                return Ok(()); // Skip this item
            }
        };
        
        // Verify quality requirements one more time with decision engine
        if let Some(decision_engine) = &self.decision_engine {
            if decision_engine.evaluate_release(&release).is_none() {
                warn!("RSS item '{}' failed quality check during processing", item.title);
                return Ok(());
            }
        }
        
        // Create queue item
        let mut queue_item = QueueItem::new(
            movie.id,
            Uuid::new_v4(), // Release ID (generated for RSS items)
            item.title.clone(),
            item.url.clone(),
        );
        
        // Set optional queue item properties
        if let Some(size) = item.size {
            queue_item.size_bytes = Some(size as i64);
        }
        
        // Set priority based on feed configuration or movie preferences
        queue_item.priority = QueuePriority::Normal;
        
        // Set category from feed if specified
        if !feed.categories.is_empty() {
            queue_item.category = Some(feed.categories[0].clone());
        }
        
        // Add to download queue
        self.queue_repository.add_queue_item(&queue_item).await.map_err(|e| {
            error!("Failed to add RSS item to queue: {}", e);
            e
        })?;
        
        info!("Successfully queued RSS item '{}' for movie '{}'", item.title, movie.title);
        
        // Emit event with actual movie ID
        if let Some(bus) = &self.event_bus {
            let _ = bus.publish(SystemEvent::DownloadQueued {
                movie_id: movie.id,
                release_id: queue_item.release_id,
                download_url: item.url.clone(),
                title: item.title.clone(),
            }).await;
        }
        
        Ok(())
    }
    
    /// Run calendar monitoring loop
    async fn run_calendar_monitor(&self) {
        let mut check_interval = interval(Duration::from_secs(self.config.calendar_interval_seconds));
        
        loop {
            check_interval.tick().await;
            
            let monitor = self.monitor.read().await;
            let searchable = monitor.get_searchable_entries();
            
            for entry in searchable {
                info!("Calendar trigger for movie: {}", entry.title);
                
                // Perform indexer search for calendar-triggered movie
                self.search_movie(entry).await;
            }
        }
    }
    
    /// Search for a movie based on calendar entry
    async fn search_movie(&self, entry: &CalendarEntry) {
        debug!("Searching for movie: {}", entry.title);
        
        // Start progress tracking
        let progress_id = if let Some(tracker) = &self.progress_tracker {
            Some(tracker.start_operation(
                OperationType::IndexerSearch,
                format!("Calendar search: {}", entry.title)
            ).await)
        } else {
            None
        };
        
        let search_result = self.perform_movie_search(entry).await;
        
        // Complete or fail progress based on result
        if let (Some(tracker), Some(id)) = (&self.progress_tracker, progress_id) {
            match search_result {
                Ok(Some(release_title)) => {
                    tracker.complete_operation(id, format!("Found and queued: {}", release_title)).await;
                }
                Ok(None) => {
                    tracker.complete_operation(id, "No suitable releases found").await;
                }
                Err(e) => {
                    tracker.fail_operation(id, e.to_string()).await;
                }
            }
        }
    }
    
    /// Perform the actual movie search and queue the best match
    async fn perform_movie_search(&self, entry: &CalendarEntry) -> Result<Option<String>> {
        // 1. Build search query using the movie title
        let search_request = SearchRequest::for_movie_title(&entry.title)
            .with_limit(50) // Get up to 50 results to choose from
            .with_min_seeders(1); // Require at least 1 seeder
        
        // 2. Search indexers
        let search_response = match self.indexer_client.search(&search_request).await {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to search indexers for movie '{}': {}", entry.title, e);
                return Err(e.into());
            }
        };
        
        if search_response.results.is_empty() {
            info!("No search results found for movie: {}", entry.title);
            return Ok(None);
        }
        
        info!("Found {} search results for movie: {}", search_response.results.len(), entry.title);
        
        // 3. Convert search results to Release objects for evaluation
        let releases: Vec<Release> = search_response.results.into_iter().map(|result| {
            let mut release = Release::from_title(result.title.clone(), result.download_url);
            
            // Set additional properties from search result
            if let Some(size) = result.size {
                if size > 0 {
                    release = release.with_size(size as u64);
                }
            }
            if let Some(seeders) = result.seeders {
                if seeders > 0 {
                    release = release.with_seeders(seeders as u32);
                }
            }
            if let Some(leechers) = result.leechers {
                if leechers > 0 {
                    release = release.with_leechers(leechers as u32);
                }
            }
            if let Some(publish_date) = result.publish_date {
                let age = Utc::now() - publish_date;
                let age_hours = age.num_hours().max(0) as u32;
                release = release.with_age_hours(age_hours);
            }
            if result.freeleech == Some(true) {
                release = release.with_freeleech(true);
            }
            
            release
        }).collect();
        
        // 4. Evaluate results using decision engine if available
        let best_release = if let Some(decision_engine) = &self.decision_engine {
            match decision_engine.select_best_release(releases) {
                Some(release) => release,
                None => {
                    info!("No releases met quality requirements for movie: {}", entry.title);
                    return Ok(None);
                }
            }
        } else {
            // No decision engine - just pick the first release
            warn!("No decision engine configured, selecting first release");
            match releases.into_iter().next() {
                Some(release) => release,
                None => {
                    error!("No releases available after conversion");
                    return Ok(None);
                }
            }
        };
        
        info!("Selected release for movie '{}': {}", entry.title, best_release.title);
        
        // 5. Queue the best match
        self.queue_movie_release(entry, &best_release).await.map(|_| Some(best_release.title))
    }
    
    /// Queue a movie release for download
    async fn queue_movie_release(&self, entry: &CalendarEntry, release: &Release) -> Result<()> {
        // Create queue item
        let mut queue_item = QueueItem::new(
            entry.movie_id,
            Uuid::new_v4(), // Release ID (generated for calendar searches)
            release.title.clone(),
            release.download_url.clone(),
        );
        
        // Set optional queue item properties from release
        if let Some(size) = release.size {
            queue_item.size_bytes = Some(size as i64);
        }
        
        // Set priority for calendar-triggered searches
        queue_item.priority = QueuePriority::High;
        
        // Add to download queue
        self.queue_repository.add_queue_item(&queue_item).await.map_err(|e| {
            error!("Failed to queue calendar release '{}' for movie '{}': {}", release.title, entry.title, e);
            e
        })?;
        
        info!("Successfully queued calendar release '{}' for movie '{}'", release.title, entry.title);
        
        // Emit event
        if let Some(bus) = &self.event_bus {
            let _ = bus.publish(SystemEvent::DownloadQueued {
                movie_id: entry.movie_id,
                release_id: queue_item.release_id,
                download_url: release.download_url.clone(),
                title: release.title.clone(),
            }).await;
        }
        
        Ok(())
    }
    
    /// Add an RSS feed
    pub async fn add_feed(&self, feed: RssFeed) -> Result<()> {
        let mut monitor = self.monitor.write().await;
        monitor.add_feed(feed);
        Ok(())
    }
    
    /// Remove an RSS feed
    pub async fn remove_feed(&self, id: Uuid) -> Result<()> {
        let mut monitor = self.monitor.write().await;
        monitor.remove_feed(id);
        Ok(())
    }
    
    /// Get all RSS feeds
    pub async fn get_feeds(&self) -> Vec<RssFeed> {
        let monitor = self.monitor.read().await;
        // Clone the feeds since we can't return references
        monitor.get_feeds().into_iter().cloned().collect()
    }
    
    /// Add a calendar entry
    pub async fn add_calendar_entry(&self, entry: CalendarEntry) -> Result<()> {
        let mut monitor = self.monitor.write().await;
        monitor.add_calendar_entry(entry);
        Ok(())
    }
    
    /// Get upcoming calendar entries
    pub async fn get_upcoming(&self, days: i64) -> Vec<CalendarEntry> {
        let monitor = self.monitor.read().await;
        monitor.get_upcoming_entries(days)
            .into_iter()
            .cloned()
            .collect()
    }
    
    /// Test an RSS feed
    pub async fn test_feed(&self, url: &str) -> Result<Vec<RssItem>> {
        let client = reqwest::Client::new();
        let response = client.get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| RadarrError::NetworkError {
                message: format!("Failed to fetch RSS feed: {}", e),
            })?;
        
        let content = response.text().await
            .map_err(|e| RadarrError::NetworkError {
                message: format!("Failed to read RSS content: {}", e),
            })?;
        
        RssParser::parse_feed(&content)
    }
}