//! RSS monitoring service for automated discovery

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use radarr_core::{
    Result, RadarrError,
    rss::{RssFeed, RssItem, RssMonitor, RssParser, CalendarEntry},
    events::{EventBus, SystemEvent},
    progress::{ProgressTracker, OperationType},
};
use radarr_indexers::IndexerClient;
use radarr_infrastructure::DatabasePool;
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
}

impl RssService {
    /// Create new RSS service
    pub fn new(
        config: RssServiceConfig,
        indexer_client: Arc<dyn IndexerClient + Send + Sync>,
        database_pool: DatabasePool,
    ) -> Self {
        Self {
            config,
            monitor: Arc::new(RwLock::new(RssMonitor::new())),
            indexer_client,
            database_pool,
            event_bus: None,
            progress_tracker: None,
            processed_items: Arc::new(RwLock::new(Vec::new())),
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
        
        // TODO: Check if movie exists and is monitored
        // TODO: Check quality requirements
        
        true
    }
    
    /// Process an RSS item
    async fn process_rss_item(&self, item: &RssItem, feed: &RssFeed) -> Result<()> {
        info!("Processing RSS item: {}", item.title);
        
        // TODO: Parse release name to identify movie
        // TODO: Check if quality meets requirements
        // TODO: Add to download queue
        
        // Emit event
        if let Some(bus) = &self.event_bus {
            let _ = bus.publish(SystemEvent::DownloadQueued {
                movie_id: Uuid::new_v4(), // TODO: Get actual movie ID
                release_id: Uuid::new_v4(),
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
                
                // TODO: Trigger movie search
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
        
        // TODO: Implement actual movie search
        // 1. Build search query
        // 2. Search indexers
        // 3. Evaluate results
        // 4. Queue best match
        
        // Complete progress
        if let (Some(tracker), Some(id)) = (&self.progress_tracker, progress_id) {
            tracker.complete_operation(id, "Search completed").await;
        }
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
        let mut feeds = Vec::new();
        for _ in 0..0 { // TODO: RssMonitor needs a get_feeds method
            // feeds.push(feed.clone());
        }
        feeds
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