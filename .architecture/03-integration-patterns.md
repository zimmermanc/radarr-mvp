# Integration Patterns & External Systems

**Last Updated**: 2025-08-19  
**Focus**: External system integrations and communication patterns  
**Status**: Planning for implementation based on official Radarr analysis  

## Integration Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           External Integration Map                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
│  │   TMDB API  │    │   HDBits    │    │  Prowlarr   │    │   Jackett   │      │
│  │ (Working ✅) │    │ (Working ✅) │    │(Planned ❌)  │    │(Planned ❌)  │      │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘      │
│          │                   │                   │                   │         │
│          └───────────────────┼───────────────────┼───────────────────┘         │
│                              │                   │                             │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                      Radarr MVP Core                                       ││
│  │                                                                             ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        ││
│  │  │  Metadata   │  │   Scene     │  │   Search    │  │  Download   │        ││
│  │  │  Service    │  │  Analysis   │  │   Engine    │  │   Queue     │        ││
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                              │                   │                             │
│          ┌───────────────────┼───────────────────┼───────────────────┐         │
│          │                   │                   │                   │         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
│  │qBittorrent  │    │   SABnzbd   │    │    Plex     │    │  Jellyfin   │      │
│  │(Planned ❌)  │    │(Planned ❌)  │    │(Future 🔮)  │    │(Future 🔮)  │      │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘      │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Metadata Integration (TMDB)

### Current Implementation ✅

```rust
#[derive(Debug, Clone)]
pub struct TmdbClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    rate_limiter: RateLimiter,
}

impl TmdbClient {
    pub async fn search_movies(&self, query: &str, year: Option<u16>) -> Result<Vec<TmdbMovie>> {
        self.rate_limiter.acquire().await;
        
        let mut params = vec![
            ("api_key", self.api_key.as_str()),
            ("query", query),
        ];
        
        if let Some(year) = year {
            params.push(("year", &year.to_string()));
        }
        
        let response = self.client
            .get(&format!("{}/search/movie", self.base_url))
            .query(&params)
            .send()
            .await?;
            
        let search_result: TmdbSearchResponse = response.json().await?;
        Ok(search_result.results)
    }
    
    pub async fn get_movie_details(&self, tmdb_id: u32) -> Result<TmdbMovieDetails> {
        self.rate_limiter.acquire().await;
        
        let response = self.client
            .get(&format!("{}/movie/{}", self.base_url, tmdb_id))
            .query(&[("api_key", &self.api_key)])
            .send()
            .await?;
            
        response.json().await.map_err(Into::into)
    }
    
    pub async fn get_movie_credits(&self, tmdb_id: u32) -> Result<TmdbCredits> {
        self.rate_limiter.acquire().await;
        
        let response = self.client
            .get(&format!("{}/movie/{}/credits", self.base_url, tmdb_id))
            .query(&[("api_key", &self.api_key)])
            .send()
            .await?;
            
        response.json().await.map_err(Into::into)
    }
}
```

### Rate Limiting Implementation

```rust
pub struct RateLimiter {
    semaphore: Semaphore,
    last_request: Arc<Mutex<Instant>>,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            semaphore: Semaphore::new(requests_per_second as usize),
            last_request: Arc::new(Mutex::new(Instant::now())),
            min_interval: Duration::from_millis(1000 / requests_per_second as u64),
        }
    }
    
    pub async fn acquire(&self) -> RateLimitGuard {
        let _permit = self.semaphore.acquire().await.unwrap();
        
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        
        if elapsed < self.min_interval {
            tokio::time::sleep(self.min_interval - elapsed).await;
        }
        
        *last = Instant::now();
        RateLimitGuard { _permit }
    }
}
```

## Scene Group Analysis (HDBits)

### Current Implementation ✅

```rust
pub struct HDBitsSessionAnalyzer {
    client: Client,
    session_cookies: String,
    config: HDBitsSessionConfig,
}

impl HDBitsSessionAnalyzer {
    pub async fn analyze_comprehensive(&self) -> Result<SceneGroupAnalysisReport> {
        let mut scene_groups = HashMap::new();
        
        // Analyze each category with rate limiting
        for category in &["Movies", "TV", "Documentaries"] {
            println!("Analyzing category: {}", category);
            
            let releases = self.browse_category(category, 3).await?;
            self.process_releases(&releases, &mut scene_groups).await?;
            
            // Conservative rate limiting between categories
            tokio::time::sleep(Duration::from_secs(35)).await;
        }
        
        let analysis_report = self.generate_reputation_report(scene_groups).await?;
        self.export_analysis_results(&analysis_report).await?;
        
        Ok(analysis_report)
    }
    
    async fn browse_category(&self, category: &str, max_pages: u32) -> Result<Vec<Release>> {
        let mut releases = Vec::new();
        
        for page in 1..=max_pages {
            let page_releases = self.fetch_category_page(category, page).await?;
            releases.extend(page_releases);
            
            // Rate limiting between pages
            if page < max_pages {
                tokio::time::sleep(Duration::from_secs(35)).await;
            }
        }
        
        Ok(releases)
    }
    
    fn calculate_reputation_score(&self, group_data: &SceneGroupData) -> f64 {
        let seeder_health_score = self.calculate_seeder_health(group_data);
        let internal_ratio = group_data.internal_releases as f64 / group_data.total_releases as f64;
        let completion_rate = group_data.completed_downloads as f64 / group_data.total_downloads as f64;
        let quality_consistency = self.calculate_quality_consistency(group_data);
        let recency_score = self.calculate_recency_score(group_data);
        let category_diversity = group_data.categories.len() as f64 / 10.0; // Normalized
        let volume_score = (group_data.total_releases as f64).ln() / 10.0; // Log scale
        let size_appropriateness = self.calculate_size_appropriateness(group_data);
        
        // Weighted combination (totals 100%)
        (seeder_health_score * 0.25) +
        (internal_ratio * 100.0 * 0.20) +
        (completion_rate * 100.0 * 0.15) +
        (quality_consistency * 0.12) +
        (recency_score * 0.10) +
        (category_diversity * 100.0 * 0.08) +
        (volume_score * 100.0 * 0.05) +
        (size_appropriateness * 0.05)
    }
}
```

### Scene Group Integration Pattern

```rust
pub trait SceneGroupAnalyzer: Send + Sync {
    async fn get_reputation(&self, group_name: &str) -> Option<SceneGroupReputation>;
    async fn refresh_analysis(&self) -> Result<(), AnalysisError>;
    async fn get_quality_tier(&self, group_name: &str) -> QualityTier;
}

#[derive(Debug, Clone)]
pub struct SceneGroupReputation {
    pub group_name: String,
    pub reputation_score: f64,  // 0-100
    pub quality_tier: QualityTier,
    pub confidence_level: ConfidenceLevel,
    pub last_analyzed: DateTime<Utc>,
    pub release_count: u32,
    pub categories_covered: Vec<String>,
}

impl SceneGroupReputation {
    pub fn influences_decision(&self, threshold: f64) -> bool {
        self.reputation_score >= threshold && 
        self.confidence_level != ConfidenceLevel::Low
    }
}
```

## Indexer Integration Patterns

### Prowlarr Integration (Planned)

```rust
pub struct ProwlarrClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    rate_limiter: RateLimiter,
}

impl IndexerClient for ProwlarrClient {
    async fn search(&self, criteria: SearchCriteria) -> Result<Vec<Release>> {
        self.rate_limiter.acquire().await;
        
        let search_request = ProwlarrSearchRequest {
            query: criteria.build_query(),
            categories: criteria.categories.clone(),
            indexer_ids: criteria.indexer_ids.clone(),
        };
        
        let response = self.client
            .get(&format!("{}/api/v1/search", self.base_url))
            .header("X-Api-Key", &self.api_key)
            .query(&search_request)
            .send()
            .await?;
            
        let search_results: Vec<ProwlarrRelease> = response.json().await?;
        Ok(search_results.into_iter().map(Into::into).collect())
    }
    
    async fn get_indexers(&self) -> Result<Vec<IndexerInfo>> {
        self.rate_limiter.acquire().await;
        
        let response = self.client
            .get(&format!("{}/api/v1/indexer", self.base_url))
            .header("X-Api-Key", &self.api_key)
            .send()
            .await?;
            
        response.json().await.map_err(Into::into)
    }
    
    async fn test_indexer(&self, indexer_id: u32) -> Result<TestResult> {
        let response = self.client
            .post(&format!("{}/api/v1/indexer/{}/test", self.base_url, indexer_id))
            .header("X-Api-Key", &self.api_key)
            .send()
            .await?;
            
        response.json().await.map_err(Into::into)
    }
}
```

### Jackett Integration (Planned)

```rust
pub struct JackettClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    indexer_configs: HashMap<String, JackettIndexerConfig>,
}

impl IndexerClient for JackettClient {
    async fn search(&self, criteria: SearchCriteria) -> Result<Vec<Release>> {
        let mut all_releases = Vec::new();
        
        // Search across all configured indexers
        for (indexer_id, config) in &self.indexer_configs {
            if !config.enabled {
                continue;
            }
            
            let releases = self.search_indexer(indexer_id, &criteria).await?;
            all_releases.extend(releases);
            
            // Rate limit between indexers
            tokio::time::sleep(config.rate_limit).await;
        }
        
        Ok(all_releases)
    }
}

impl JackettClient {
    async fn search_indexer(&self, indexer_id: &str, criteria: &SearchCriteria) -> Result<Vec<Release>> {
        let torznab_params = TorznabParams {
            apikey: &self.api_key,
            t: "movie", // or "search"
            q: &criteria.build_query(),
            cat: &criteria.categories.join(","),
            imdbid: criteria.imdb_id.as_deref(),
        };
        
        let response = self.client
            .get(&format!("{}/api/v2.0/indexers/{}/results/torznab", self.base_url, indexer_id))
            .query(&torznab_params)
            .send()
            .await?;
            
        let xml_content = response.text().await?;
        let releases = self.parse_torznab_response(&xml_content)?;
        
        Ok(releases)
    }
    
    fn parse_torznab_response(&self, xml: &str) -> Result<Vec<Release>> {
        // Parse Torznab XML format
        // Implementation would use xml parsing library
        todo!("Implement Torznab XML parsing")
    }
}
```

### Generic Indexer Interface

```rust
#[async_trait]
pub trait IndexerClient: Send + Sync {
    async fn search(&self, criteria: SearchCriteria) -> Result<Vec<Release>>;
    async fn get_capabilities(&self) -> Result<IndexerCapabilities>;
    async fn test_connection(&self) -> Result<TestResult>;
}

#[derive(Debug, Clone)]
pub struct SearchCriteria {
    pub title: String,
    pub year: Option<u16>,
    pub imdb_id: Option<String>,
    pub categories: Vec<String>,
    pub minimum_seeders: Option<u32>,
    pub maximum_age_days: Option<u32>,
    pub indexer_ids: Option<Vec<u32>>,
}

impl SearchCriteria {
    pub fn build_query(&self) -> String {
        let mut query = self.title.clone();
        
        if let Some(year) = self.year {
            query.push_str(&format!(" {}", year));
        }
        
        query
    }
    
    pub fn for_movie(movie: &Movie) -> Self {
        Self {
            title: movie.title.clone(),
            year: movie.year,
            imdb_id: movie.metadata.imdb_id.clone(),
            categories: vec!["2000".to_string()], // Movies category
            minimum_seeders: Some(5),
            maximum_age_days: Some(365),
            indexer_ids: None,
        }
    }
}
```

## Download Client Integration

### qBittorrent Client (Planned)

```rust
pub struct QBittorrentClient {
    client: reqwest::Client,
    base_url: String,
    session: Arc<Mutex<Option<QBittorrentSession>>>,
    credentials: QBittorrentCredentials,
}

impl DownloadClient for QBittorrentClient {
    async fn add_torrent(&self, request: AddTorrentRequest) -> Result<DownloadId> {
        self.ensure_authenticated().await?;
        
        let form = multipart::Form::new()
            .text("urls", request.magnet_link)
            .text("category", request.category)
            .text("paused", "false")
            .text("sequentialDownload", "false")
            .text("firstLastPiecePrio", "false");
            
        let response = self.client
            .post(&format!("{}/api/v2/torrents/add", self.base_url))
            .multipart(form)
            .send()
            .await?;
            
        if response.status().is_success() {
            Ok(DownloadId::new(&request.magnet_link))
        } else {
            Err(DownloadClientError::AddFailed(response.text().await?))
        }
    }
    
    async fn get_torrent_list(&self) -> Result<Vec<TorrentInfo>> {
        self.ensure_authenticated().await?;
        
        let response = self.client
            .get(&format!("{}/api/v2/torrents/info", self.base_url))
            .send()
            .await?;
            
        response.json().await.map_err(Into::into)
    }
    
    async fn get_torrent_status(&self, download_id: DownloadId) -> Result<DownloadStatus> {
        let torrents = self.get_torrent_list().await?;
        
        for torrent in torrents {
            if torrent.hash == download_id.value() {
                return Ok(self.map_torrent_status(&torrent));
            }
        }
        
        Err(DownloadClientError::NotFound(download_id))
    }
}

impl QBittorrentClient {
    async fn ensure_authenticated(&self) -> Result<()> {
        let mut session = self.session.lock().await;
        
        if session.is_none() || session.as_ref().unwrap().is_expired() {
            let new_session = self.authenticate().await?;
            *session = Some(new_session);
        }
        
        Ok(())
    }
    
    async fn authenticate(&self) -> Result<QBittorrentSession> {
        let form = [
            ("username", &self.credentials.username),
            ("password", &self.credentials.password),
        ];
        
        let response = self.client
            .post(&format!("{}/api/v2/auth/login", self.base_url))
            .form(&form)
            .send()
            .await?;
            
        if response.status().is_success() {
            // Extract session cookie
            let cookies = response.cookies();
            Ok(QBittorrentSession::new(cookies))
        } else {
            Err(DownloadClientError::AuthenticationFailed)
        }
    }
}
```

### SABnzbd Client (Planned)

```rust
pub struct SabnzbdClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl DownloadClient for SabnzbdClient {
    async fn add_nzb(&self, request: AddNzbRequest) -> Result<DownloadId> {
        let params = [
            ("mode", "addurl"),
            ("name", &request.nzb_url),
            ("cat", &request.category),
            ("priority", &request.priority.to_string()),
            ("apikey", &self.api_key),
            ("output", "json"),
        ];
        
        let response = self.client
            .post(&format!("{}/api", self.base_url))
            .form(&params)
            .send()
            .await?;
            
        let result: SabnzbdAddResponse = response.json().await?;
        
        if result.status {
            Ok(DownloadId::new(&result.nzo_id))
        } else {
            Err(DownloadClientError::AddFailed(result.error.unwrap_or_default()))
        }
    }
    
    async fn get_queue(&self) -> Result<Vec<QueueItem>> {
        let params = [
            ("mode", "queue"),
            ("apikey", &self.api_key),
            ("output", "json"),
        ];
        
        let response = self.client
            .get(&format!("{}/api", self.base_url))
            .query(&params)
            .send()
            .await?;
            
        let queue_response: SabnzbdQueueResponse = response.json().await?;
        Ok(queue_response.queue.slots)
    }
    
    async fn get_history(&self) -> Result<Vec<HistoryItem>> {
        let params = [
            ("mode", "history"),
            ("apikey", &self.api_key),
            ("output", "json"),
        ];
        
        let response = self.client
            .get(&format!("{}/api", self.base_url))
            .query(&params)
            .send()
            .await?;
            
        let history_response: SabnzbdHistoryResponse = response.json().await?;
        Ok(history_response.history.slots)
    }
}
```

### Generic Download Client Interface

```rust
#[async_trait]
pub trait DownloadClient: Send + Sync {
    async fn add_download(&self, request: AddDownloadRequest) -> Result<DownloadId>;
    async fn get_download_status(&self, id: DownloadId) -> Result<DownloadStatus>;
    async fn get_active_downloads(&self) -> Result<Vec<ActiveDownload>>;
    async fn get_completed_downloads(&self) -> Result<Vec<CompletedDownload>>;
    async fn pause_download(&self, id: DownloadId) -> Result<()>;
    async fn resume_download(&self, id: DownloadId) -> Result<()>;
    async fn remove_download(&self, id: DownloadId, delete_files: bool) -> Result<()>;
    async fn test_connection(&self) -> Result<TestResult>;
}

#[derive(Debug, Clone)]
pub enum AddDownloadRequest {
    Torrent {
        magnet_link: String,
        category: String,
        priority: Priority,
    },
    Nzb {
        nzb_url: String,
        category: String,
        priority: Priority,
    },
}

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Queued,
    Downloading { progress: f32, eta: Option<Duration> },
    Paused { reason: Option<String> },
    Completed { output_path: PathBuf },
    Failed { error: String },
    Removed,
}
```

## Media Server Integration (Future)

### Plex Integration Pattern

```rust
pub struct PlexClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl MediaServerClient for PlexClient {
    async fn refresh_library(&self, library_id: u32) -> Result<()> {
        let response = self.client
            .put(&format!("{}/library/sections/{}/refresh", self.base_url, library_id))
            .header("X-Plex-Token", &self.token)
            .send()
            .await?;
            
        if response.status().is_success() {
            Ok(())
        } else {
            Err(MediaServerError::RefreshFailed)
        }
    }
    
    async fn get_library_sections(&self) -> Result<Vec<LibrarySection>> {
        let response = self.client
            .get(&format!("{}/library/sections", self.base_url))
            .header("X-Plex-Token", &self.token)
            .send()
            .await?;
            
        let sections_response: PlexSectionsResponse = response.json().await?;
        Ok(sections_response.media_container.directory)
    }
    
    async fn scan_library_path(&self, path: &Path) -> Result<()> {
        // Trigger scan for specific path
        let params = [("path", path.to_string_lossy())];
        
        let response = self.client
            .post(&format!("{}/library/sections/all/refresh", self.base_url))
            .header("X-Plex-Token", &self.token)
            .query(&params)
            .send()
            .await?;
            
        if response.status().is_success() {
            Ok(())
        } else {
            Err(MediaServerError::ScanFailed)
        }
    }
}
```

## Error Handling & Resilience

### Circuit Breaker Pattern

```rust
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_threshold: u32,
    timeout: Duration,
    retry_timeout: Duration,
}

#[derive(Debug)]
enum CircuitState {
    Closed { failure_count: u32 },
    Open { opened_at: Instant },
    HalfOpen,
}

impl CircuitBreaker {
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error,
    {
        let mut state = self.state.lock().await;
        
        match &*state {
            CircuitState::Open { opened_at } => {
                if opened_at.elapsed() > self.retry_timeout {
                    *state = CircuitState::HalfOpen;
                } else {
                    return Err(CircuitBreakerError::CircuitOpen);
                }
            }
            _ => {}
        }
        
        drop(state);
        
        match operation() {
            Ok(result) => {
                let mut state = self.state.lock().await;
                *state = CircuitState::Closed { failure_count: 0 };
                Ok(result)
            }
            Err(error) => {
                let mut state = self.state.lock().await;
                match &*state {
                    CircuitState::Closed { failure_count } => {
                        let new_failure_count = failure_count + 1;
                        if new_failure_count >= self.failure_threshold {
                            *state = CircuitState::Open { opened_at: Instant::now() };
                        } else {
                            *state = CircuitState::Closed { failure_count: new_failure_count };
                        }
                    }
                    CircuitState::HalfOpen => {
                        *state = CircuitState::Open { opened_at: Instant::now() };
                    }
                    _ => {}
                }
                Err(CircuitBreakerError::OperationFailed(error))
            }
        }
    }
}
```

### Retry Strategy

```rust
pub struct RetryStrategy {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
}

impl RetryStrategy {
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Result<T, E>,
        E: std::error::Error + RetryableError,
    {
        let mut attempts = 0;
        let mut delay = self.base_delay;
        
        loop {
            attempts += 1;
            
            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempts >= self.max_attempts || !error.is_retryable() {
                        return Err(error);
                    }
                    
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis((delay.as_millis() as f64 * self.backoff_multiplier) as u64),
                        self.max_delay,
                    );
                }
            }
        }
    }
}

pub trait RetryableError {
    fn is_retryable(&self) -> bool;
}
```

## Configuration Management

### External Service Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ExternalServicesConfig {
    pub tmdb: TmdbConfig,
    pub hdbits: HDBitsConfig,
    pub indexers: IndexersConfig,
    pub download_clients: DownloadClientsConfig,
    pub media_servers: Option<MediaServersConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexersConfig {
    pub prowlarr: Option<ProwlarrConfig>,
    pub jackett: Option<JackettConfig>,
    pub direct_indexers: Vec<DirectIndexerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProwlarrConfig {
    pub url: String,
    pub api_key: String,
    pub sync_enabled: bool,
    pub sync_interval: Duration,
    pub rate_limit: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadClientsConfig {
    pub qbittorrent: Option<QBittorrentConfig>,
    pub sabnzbd: Option<SabnzbdConfig>,
    pub default_client: String,
    pub category_mappings: HashMap<String, String>,
}
```

This integration pattern provides a comprehensive foundation for integrating with all major external systems that Radarr needs to interact with, following established patterns from the official implementation while leveraging Rust's type safety and async capabilities.