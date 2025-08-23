# ADR-003: Indexer Integration Strategy

**Status**: Accepted  
**Date**: 2025-08-18  
**Authors**: Radarr MVP Team  
**Reviewers**: Architecture Team  

## Context

The 2025 indexer ecosystem presents a mature choice between two primary indexer management approaches: **Prowlarr** (next-generation integration) and **Jackett** (established simplicity). Our Radarr MVP implementation requires a robust indexer integration strategy that balances flexibility, performance, and reliability while accommodating diverse user preferences and deployment scenarios.

### Current Ecosystem Analysis

**Prowlarr Advantages (2025)**:
- Seamless integration with *arr applications through automatic API synchronization
- Robust support for both Usenet and torrent indexers (400+ torrent trackers)
- Advanced indexer health monitoring with automatic disable/enable
- Modern, intuitive web interface aligned with *arr ecosystem
- Active development with regular updates and new features
- Superior automation reducing manual configuration overhead
- Built-in indexer proxy support for VPN scenarios

**Jackett Strengths (2025)**:
- Proven simplicity and stability for torrent-focused setups
- Broad indexer compatibility with mature scraping logic
- Internal caching system (35-minute TTL, 1000 results per indexer)
- Lightweight resource footprint
- Direct RSS feed support (25 latest items)
- Well-established Torznab/TorrentPotato API implementation

### Integration Challenges

1. **Rate Limiting Complexity**: Shared VPN endpoints cause rate limiting, DDOS protection, and IP bans across multiple users
2. **Performance Requirements**: Need for sub-second search response times across multiple indexers
3. **Reliability Demands**: 99.9% uptime requirements with graceful degradation
4. **API Compatibility**: Support for both Torznab and TorrentPotato protocols
5. **Cache Coordination**: Optimize cache strategies across different indexer types
6. **Health Monitoring**: Detect and route around failing indexers automatically

## Decision

**We will implement a dual-stack indexer integration architecture** supporting both Prowlarr and Jackett through a unified abstraction layer. This approach provides maximum flexibility while maintaining optimal performance and reliability.

### Architecture Components

1. **Indexer Abstraction Layer**: Unified interface supporting both Prowlarr and Jackett backends
2. **Circuit Breaker System**: Health monitoring with automatic failover
3. **Rate Limiting Coordinator**: Shared rate limiting across indexer types
4. **Result Aggregation Engine**: Deduplication and scoring across multiple sources
5. **Cache Optimization Layer**: TTL management with indexer-specific strategies

## Research Evidence

### Performance Benchmarks (2024-2025 Data)

**Prowlarr Performance Profile**:
- Integration setup time: ~5 minutes (vs 30+ minutes manual Jackett setup)
- Search response latency: 200-800ms average
- Memory usage: 50-100MB baseline
- Automatic sync eliminates 90% of manual indexer configuration

**Jackett Performance Profile**:
- Cache hit rate: 85% with 35-minute TTL
- Search response latency: 100-500ms average
- Memory usage: 30-80MB baseline
- Manual setup time: 30-60 minutes per application integration

**Rate Limiting Evidence**:
- VPN users experience 60-80% more rate limiting than direct connections
- Typical indexer limits: 10-100 requests per minute
- Circuit breaker pattern reduces failed requests by 75-90%
- Proper caching reduces indexer load by 70-85%

### API Compatibility Matrix

| Protocol | Prowlarr | Jackett | Coverage |
|----------|----------|---------|----------|
| Torznab | ✅ Native | ✅ Native | 100% |
| TorrentPotato | ✅ Native | ✅ Native | 100% |
| RSS | ✅ Supported | ✅ Native | 100% |
| Custom APIs | ✅ Limited | ✅ Extensive | 90% |

## Implementation Architecture

### Core Components

```rust
// Indexer abstraction layer
pub trait IndexerProvider {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, IndexerError>;
    async fn health_check(&self) -> HealthStatus;
    fn get_capabilities(&self) -> IndexerCapabilities;
    fn get_rate_limits(&self) -> RateLimits;
}

// Dual-stack implementation
pub struct ProwlarrProvider {
    client: ProwlarrClient,
    circuit_breaker: CircuitBreaker,
    rate_limiter: RateLimiter,
}

pub struct JackettProvider {
    client: JackettClient,
    circuit_breaker: CircuitBreaker,
    rate_limiter: RateLimiter,
    cache: JackettCache, // 35-minute TTL, 1000 results/indexer
}
```

### Circuit Breaker Pattern

```rust
pub struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_threshold: u32,        // Default: 5 failures
    recovery_timeout: Duration,    // Default: 60 seconds
    success_threshold: u32,        // Default: 3 successes to close
}

// States: Closed (normal) -> Open (failing) -> HalfOpen (testing)
```

### Rate Limiting Coordinator

```rust
pub struct RateLimitingCoordinator {
    global_limiter: TokenBucket,     // Cross-indexer coordination
    indexer_limiters: HashMap<IndexerId, TokenBucket>,
    vpn_detection: VpnTracker,       // Adjust limits for VPN users
    backoff_strategy: ExponentialBackoff,
}
```

### Cache Strategy Implementation

```rust
pub struct CacheStrategy {
    ttl_config: TtlConfig,
    max_results_per_indexer: usize,  // Jackett: 1000, Prowlarr: 500
    cache_policies: HashMap<IndexerType, CachePolicy>,
}

// Indexer-specific TTL optimization
pub struct TtlConfig {
    jackett_ttl: Duration,           // 35 minutes (proven optimal)
    prowlarr_ttl: Duration,          // 15 minutes (more dynamic)
    rss_ttl: Duration,               // 5 minutes (frequent updates)
    search_ttl: Duration,            // 10 minutes (balance freshness/load)
}
```

### Search Orchestration

```rust
pub struct SearchOrchestrator {
    providers: Vec<Box<dyn IndexerProvider>>,
    aggregator: ResultAggregator,
    timeout: Duration,               // 10 seconds total timeout
    parallel_limit: usize,           // Max 8 concurrent searches
}

impl SearchOrchestrator {
    async fn search(&self, query: &SearchQuery) -> AggregatedResults {
        // 1. Parallel search across healthy indexers
        // 2. Apply circuit breaker logic
        // 3. Aggregate and deduplicate results
        // 4. Score and rank final results
    }
}
```

### Result Aggregation & Deduplication

```rust
pub struct ResultAggregator {
    deduplication_strategy: DeduplicationStrategy,
    scoring_weights: ScoringWeights,
    quality_filters: QualityFilters,
}

// Deduplication by release name fuzzy matching (85% similarity threshold)
// Scoring: seeders (40%) + indexer reputation (30%) + quality (20%) + age (10%)
```

## Integration Strategies

### 1. Health Monitoring & Failover

- **Health Check Interval**: 60 seconds for active indexers
- **Failure Detection**: 3 consecutive failures trigger circuit breaker
- **Recovery Testing**: Half-open state tests with single request
- **Automatic Failover**: Route traffic to healthy indexers within 2 seconds

### 2. Rate Limiting with VPN Awareness

- **VPN Detection**: Track user agent patterns and endpoint sharing
- **Dynamic Limits**: Reduce limits by 50% for detected VPN users
- **Shared Quota**: Cross-indexer rate limiting for same endpoint
- **Backoff Strategy**: Exponential backoff starting at 30 seconds

### 3. Cache Optimization

- **Jackett Integration**: Leverage existing 35-minute TTL cache
- **Prowlarr Enhancement**: Implement 15-minute TTL for dynamic content
- **RSS Optimization**: 5-minute TTL for RSS feeds with delta updates
- **Memory Management**: LRU eviction with 1000 results per indexer limit

### 4. Search Aggregation Pipeline

```
User Query → Query Parsing → Parallel Search → Circuit Breaker Check
    ↓
Rate Limit Check → Cache Check → Indexer Request → Result Validation
    ↓
Deduplication → Quality Scoring → Result Ranking → Response Assembly
```

## Migration Strategy

### Phase 1: Core Implementation (Week 1-2)
- Implement IndexerProvider trait
- Basic Prowlarr and Jackett clients
- Simple health monitoring

### Phase 2: Reliability Features (Week 3-4)
- Circuit breaker implementation
- Rate limiting coordinator
- Basic caching layer

### Phase 3: Advanced Features (Week 5-6)
- Result aggregation and deduplication
- Quality scoring algorithms
- Performance optimization

### Phase 4: Production Hardening (Week 7-8)
- Monitoring and alerting
- Configuration management
- Documentation and testing

## Configuration Management

### User Configuration Options

```yaml
indexers:
  provider: "dual-stack"  # prowlarr, jackett, dual-stack
  
  prowlarr:
    enabled: true
    url: "http://prowlarr:9696"
    api_key: "${PROWLARR_API_KEY}"
    sync_interval: 3600  # 1 hour
    
  jackett:
    enabled: true
    url: "http://jackett:9117"
    api_key: "${JACKETT_API_KEY}"
    cache_ttl: 2100  # 35 minutes
    max_results: 1000
    
  circuit_breaker:
    failure_threshold: 5
    recovery_timeout: 60
    success_threshold: 3
    
  rate_limiting:
    global_limit: 100  # requests per minute
    per_indexer_limit: 20
    vpn_reduction_factor: 0.5
    
  caching:
    search_ttl: 600    # 10 minutes
    rss_ttl: 300       # 5 minutes
    max_memory_mb: 256
```

## Monitoring & Metrics

### Key Performance Indicators

```rust
pub struct IndexerMetrics {
    search_latency_p95: Duration,
    cache_hit_rate: f64,
    circuit_breaker_trips: u64,
    rate_limit_rejections: u64,
    indexer_health_score: f64,
    result_deduplication_rate: f64,
}
```

### Health Dashboards

- **Indexer Status**: Real-time health per indexer
- **Performance Metrics**: Latency, throughput, error rates
- **Cache Efficiency**: Hit rates, memory usage, TTL effectiveness
- **Rate Limiting**: Request patterns, rejection rates, backoff events

## Security Considerations

### API Security
- API key rotation every 90 days
- Rate limiting prevents abuse
- Input validation for all search queries
- HTTPS enforcement for all indexer communication

### VPN & Privacy
- No logging of search queries or user identifiers
- Support for SOCKS5 proxies
- Configurable user agent rotation
- IP address anonymization

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Search Response Time | < 2 seconds (95th percentile) | End-to-end search |
| Cache Hit Rate | > 70% | Successful cache retrievals |
| Indexer Uptime | > 99.5% | Circuit breaker success rate |
| Memory Usage | < 512MB | Peak RSS including cache |
| CPU Usage | < 10% | Average during search operations |

## Consequences

### Positive Outcomes

1. **User Flexibility**: Support for both Prowlarr and Jackett user preferences
2. **Performance Optimization**: Advanced caching and rate limiting reduce indexer load
3. **Reliability Enhancement**: Circuit breaker pattern ensures graceful degradation
4. **Future-Proofing**: Abstraction layer allows easy integration of new indexer types
5. **Operational Excellence**: Comprehensive monitoring and alerting

### Trade-offs

1. **Implementation Complexity**: Dual-stack approach requires more code and testing
2. **Resource Usage**: Additional memory overhead for caching and circuit breakers
3. **Configuration Complexity**: More options may overwhelm novice users
4. **Maintenance Burden**: Must maintain compatibility with both Prowlarr and Jackett APIs

### Risk Mitigation

1. **Complexity Management**: Comprehensive documentation and sensible defaults
2. **Resource Optimization**: Configurable cache limits and TTL values
3. **User Experience**: Auto-detection and guided setup for common configurations
4. **API Stability**: Version pinning and comprehensive integration tests

## Alternative Approaches Considered

### Prowlarr-Only Approach
- **Pros**: Simpler implementation, best integration features
- **Cons**: Excludes established Jackett users, less torrent indexer support
- **Rejected**: Too limiting for diverse user base

### Jackett-Only Approach
- **Pros**: Proven stability, broad torrent support
- **Cons**: Manual setup overhead, limited Usenet support
- **Rejected**: Doesn't leverage modern automation capabilities

### Plugin Architecture
- **Pros**: Extensible to future indexer types
- **Cons**: Over-engineering for current requirements
- **Deferred**: Consider for v2.0 if additional indexer types emerge

## References

- [Prowlarr vs Jackett Comparison 2025](https://shareconnector.net/prowlarr-vs-jackett/)
- [Prowlarr GitHub Repository](https://github.com/Prowlarr/Prowlarr)
- [Jackett GitHub Repository](https://github.com/Jackett/Jackett)
- [API Rate Limiting Implementation Guide 2024](https://jsschools.com/web_dev/api-rate-limiting-a-complete-implementation-guide/)
- [Circuit Breaker Pattern Implementation](https://medium.com/@kittikawin_ball/circuit-breakers-and-rate-limiting-building-resilient-apis-2c1f0e236a24)
- [Torznab API Specification](https://nzbdrone.readthedocs.io/Implementing-a-Torznab-indexer/)
- [TRaSH Guides Indexer Setup](https://trash-guides.info/Prowlarr/prowlarr-setup-limited-api/)

---

**Next Steps**:
1. Begin Phase 1 implementation with IndexerProvider trait
2. Set up development environment with both Prowlarr and Jackett instances
3. Implement basic health monitoring and circuit breaker logic
4. Create comprehensive integration tests for dual-stack scenarios