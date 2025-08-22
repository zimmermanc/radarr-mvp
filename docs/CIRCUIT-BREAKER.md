# Circuit Breaker Implementation - Radarr MVP

**Implementation Date**: 2025-08-22  
**Status**: Operational  
**Coverage**: All external services protected  
**Production URL**: http://192.168.0.138:7878/

## Overview

The Radarr MVP implements a comprehensive circuit breaker system that provides fault tolerance and graceful degradation for all external service dependencies. This implementation ensures the system remains operational even when individual services experience outages or degraded performance.

## Circuit Breaker Pattern

### Core Concept
The circuit breaker pattern prevents cascading failures by monitoring service health and automatically switching to a degraded mode when services become unavailable. It operates in three states:

1. **Closed** (Normal Operation): All requests pass through normally
2. **Open** (Service Unavailable): All requests fail fast, preventing cascading failures
3. **Half-Open** (Recovery Testing): Limited requests test if the service has recovered

### State Transitions
```
Closed ──(failure threshold reached)──> Open
  ↑                                       │
  │                                       │
  └──(success)──── Half-Open ←──(timeout)┘
                        │
                        │
                   (failure limit)
                        │
                        ▼
                      Open
```

## Protected Services

### 1. TMDB API Service
**Purpose**: Movie metadata and search functionality
**Circuit Breaker Configuration**:
- Failure Threshold: 5 consecutive failures
- Recovery Timeout: 60 seconds
- Fallback: Cached data or graceful error responses

**Implementation**:
```rust
// Circuit breaker monitors TMDB API calls
// Fails open on repeated API failures
// Provides cached movie data when available
```

**Health Check**: `/api/test/circuit-breaker/tmdb`

### 2. HDBits Indexer Service
**Purpose**: Torrent indexing and scene group analysis
**Circuit Breaker Configuration**:
- Failure Threshold: 3 consecutive failures
- Recovery Timeout: 90 seconds
- Fallback: Skip torrent search, continue with other operations

**Implementation**:
```rust
// Protects against HDBits scraper failures
// Handles rate limiting and connection issues
// Continues operation without torrent search
```

**Health Check**: `/api/test/circuit-breaker/hdbits`

### 3. qBittorrent Client
**Purpose**: Download management and torrent operations
**Circuit Breaker Configuration**:
- Failure Threshold: 3 consecutive failures
- Recovery Timeout: 30 seconds
- Fallback: Queue operations for retry when service recovers

**Implementation**:
```rust
// Monitors qBittorrent Web API connectivity
// Queues download operations during outages
// Automatic retry when service recovers
```

**Health Check**: `/api/test/circuit-breaker/qbittorrent`

### 4. PostgreSQL Database
**Purpose**: Primary data storage and persistence
**Circuit Breaker Configuration**:
- Failure Threshold: 2 consecutive failures
- Recovery Timeout: 15 seconds
- Fallback: Read-only mode with cached data

**Implementation**:
```rust
// Database connection pool protection
// Prevents connection exhaustion
// Graceful degradation to cached responses
```

**Health Check**: `/api/test/circuit-breaker/database`

## Health Monitoring System

### Basic Health Check
**Endpoint**: `GET /health`
**Response**: Simple JSON indicating system status
```json
{
  "status": "healthy",
  "timestamp": "2025-08-22T10:30:00Z"
}
```

### Detailed Health Check
**Endpoint**: `GET /health/detailed`
**Response**: Comprehensive system status including circuit breaker states
```json
{
  "status": "healthy",
  "timestamp": "2025-08-22T10:30:00Z",
  "services": {
    "tmdb": {
      "status": "healthy",
      "circuit_breaker": "closed",
      "last_check": "2025-08-22T10:29:45Z",
      "failure_count": 0,
      "response_time_ms": 156
    },
    "hdbits": {
      "status": "healthy",
      "circuit_breaker": "closed",
      "last_check": "2025-08-22T10:29:50Z",
      "failure_count": 0,
      "response_time_ms": 89
    },
    "qbittorrent": {
      "status": "healthy",
      "circuit_breaker": "closed",
      "last_check": "2025-08-22T10:29:55Z",
      "failure_count": 0,
      "response_time_ms": 23
    },
    "database": {
      "status": "healthy",
      "circuit_breaker": "closed",
      "last_check": "2025-08-22T10:30:00Z",
      "failure_count": 0,
      "connection_pool_active": 5,
      "connection_pool_idle": 10
    }
  },
  "system": {
    "memory_usage_mb": 142,
    "cpu_usage_percent": 3.2,
    "uptime_seconds": 86400,
    "active_connections": 8
  }
}
```

## Testing and Demonstration

### Circuit Breaker Test Endpoints
The system provides test endpoints to demonstrate circuit breaker functionality:

#### Test TMDB Circuit Breaker
```bash
curl -X POST http://192.168.0.138:7878/api/test/circuit-breaker/tmdb
```
**Response**: Simulates TMDB service failure and demonstrates circuit breaker behavior

#### Test HDBits Circuit Breaker
```bash
curl -X POST http://192.168.0.138:7878/api/test/circuit-breaker/hdbits
```
**Response**: Forces HDBits circuit breaker to open state

#### Test qBittorrent Circuit Breaker
```bash
curl -X POST http://192.168.0.138:7878/api/test/circuit-breaker/qbittorrent
```
**Response**: Demonstrates download client circuit breaker

#### Test Database Circuit Breaker
```bash
curl -X POST http://192.168.0.138:7878/api/test/circuit-breaker/database
```
**Response**: Shows database circuit breaker protection

### Test Scenarios

#### Scenario 1: TMDB API Outage
1. TMDB service becomes unavailable
2. Circuit breaker opens after 5 failed requests
3. Movie search requests fail fast with cached data
4. System remains operational for other functions
5. Circuit breaker tests recovery every 60 seconds
6. Normal operation resumes when TMDB recovers

#### Scenario 2: qBittorrent Client Disconnection
1. qBittorrent Web API becomes unreachable
2. Circuit breaker opens after 3 failed requests
3. Download operations are queued for retry
4. Other system functions continue normally
5. Circuit breaker tests recovery every 30 seconds
6. Queued operations resume when client reconnects

#### Scenario 3: Database Connection Issues
1. PostgreSQL connection pool exhausted
2. Circuit breaker opens after 2 failed connections
3. System switches to read-only mode with cached data
4. New connections are prevented from overwhelming database
5. Circuit breaker tests recovery every 15 seconds
6. Full database operations resume when connections recover

## Architecture Implementation

### Circuit Breaker State Management
```rust
#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    Closed,    // Normal operation
    Open,      // Service unavailable
    HalfOpen,  // Testing recovery
}

pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_count: Arc<Mutex<u32>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
}
```

### Service Integration Pattern
```rust
impl ServiceClient {
    async fn execute_with_circuit_breaker<T>(
        &self,
        operation: impl Future<Output = Result<T, ServiceError>>
    ) -> Result<T, ServiceError> {
        match self.circuit_breaker.can_execute().await {
            true => {
                match operation.await {
                    Ok(result) => {
                        self.circuit_breaker.record_success().await;
                        Ok(result)
                    },
                    Err(error) => {
                        self.circuit_breaker.record_failure().await;
                        Err(error)
                    }
                }
            },
            false => {
                // Circuit is open, fail fast
                Err(ServiceError::CircuitBreakerOpen)
            }
        }
    }
}
```

### Health Check Implementation
```rust
#[derive(Serialize)]
pub struct DetailedHealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub services: HashMap<String, ServiceHealth>,
    pub system: SystemMetrics,
}

impl HealthChecker {
    pub async fn detailed_health_check(&self) -> DetailedHealthResponse {
        let mut services = HashMap::new();
        
        // Check each protected service
        services.insert("tmdb".to_string(), 
            self.check_tmdb_health().await);
        services.insert("hdbits".to_string(), 
            self.check_hdbits_health().await);
        services.insert("qbittorrent".to_string(), 
            self.check_qbittorrent_health().await);
        services.insert("database".to_string(), 
            self.check_database_health().await);
        
        DetailedHealthResponse {
            status: self.overall_status(&services),
            timestamp: Utc::now(),
            services,
            system: self.gather_system_metrics(),
        }
    }
}
```

## Production Benefits

### Improved System Reliability
- **99.9% Uptime Target**: Circuit breakers prevent cascading failures
- **Graceful Degradation**: System remains functional during partial outages
- **Fast Failure Detection**: Issues identified within seconds, not minutes
- **Automatic Recovery**: Services automatically resume when available

### Enhanced User Experience
- **Responsive Interface**: Fast failure responses prevent UI hangs
- **Partial Functionality**: Core features remain available during service outages
- **Real-time Status**: Users can see service status via health endpoints
- **Transparent Recovery**: Seamless return to full functionality

### Operational Benefits
- **Reduced Manual Intervention**: Self-healing system requires less monitoring
- **Better Monitoring**: Detailed health metrics provide operational visibility
- **Predictable Behavior**: Consistent failure handling across all services
- **Simplified Debugging**: Clear circuit breaker states aid troubleshooting

### Performance Advantages
- **Resource Protection**: Prevents resource exhaustion during outages
- **Reduced Latency**: Fast-fail responses instead of long timeouts
- **Connection Pool Management**: Database connections protected from exhaustion
- **Memory Efficiency**: Failed requests don't accumulate in memory

## Monitoring and Alerting

### Key Metrics
- **Circuit Breaker State Changes**: When circuits open/close
- **Failure Rate by Service**: Percentage of failed requests per service
- **Recovery Time**: How long services take to recover
- **Availability**: Overall system availability percentage

### Recommended Alerts
1. **Circuit Breaker Open**: Alert when any circuit opens
2. **High Failure Rate**: Alert on failure rate > 10% for any service
3. **Extended Outage**: Alert if circuit remains open > 5 minutes
4. **Recovery Success**: Notification when services recover

### Dashboard Metrics
```json
{
  "circuit_breakers": {
    "tmdb_state": "closed",
    "hdbits_state": "closed", 
    "qbittorrent_state": "closed",
    "database_state": "closed"
  },
  "failure_rates": {
    "tmdb": "0.2%",
    "hdbits": "0.5%",
    "qbittorrent": "0.1%",
    "database": "0.0%"
  },
  "response_times": {
    "tmdb_avg_ms": 156,
    "hdbits_avg_ms": 89,
    "qbittorrent_avg_ms": 23,
    "database_avg_ms": 2
  }
}
```

## Configuration

### Environment Variables
```bash
# Circuit breaker configuration
CIRCUIT_BREAKER_TMDB_THRESHOLD=5
CIRCUIT_BREAKER_TMDB_TIMEOUT_SECONDS=60
CIRCUIT_BREAKER_HDBITS_THRESHOLD=3
CIRCUIT_BREAKER_HDBITS_TIMEOUT_SECONDS=90
CIRCUIT_BREAKER_QBITTORRENT_THRESHOLD=3
CIRCUIT_BREAKER_QBITTORRENT_TIMEOUT_SECONDS=30
CIRCUIT_BREAKER_DATABASE_THRESHOLD=2
CIRCUIT_BREAKER_DATABASE_TIMEOUT_SECONDS=15

# Health check intervals
HEALTH_CHECK_INTERVAL_SECONDS=30
DETAILED_HEALTH_CACHE_SECONDS=10
```

### Runtime Configuration
Circuit breaker thresholds and timeouts can be adjusted at runtime through configuration files or environment variables without requiring application restart.

## Future Enhancements

### Planned Improvements
1. **Adaptive Thresholds**: Machine learning-based threshold adjustment
2. **Service Dependencies**: Circuit breaker chains for dependent services
3. **Metrics Export**: Prometheus metrics integration
4. **Custom Fallbacks**: Service-specific fallback strategies
5. **Load Balancing**: Multiple service endpoint support with failover

### Monitoring Integration
- **Grafana Dashboards**: Visual monitoring of circuit breaker states
- **Prometheus Metrics**: Time-series data for circuit breaker events
- **Alert Manager**: Automated alerting for circuit breaker state changes
- **Log Aggregation**: Centralized logging of circuit breaker events

## Conclusion

The circuit breaker implementation provides robust fault tolerance for the Radarr MVP, ensuring high availability and graceful degradation during service outages. This implementation follows industry best practices and provides comprehensive monitoring and testing capabilities.

**Key Achievements**:
- ✅ All external services protected by circuit breakers
- ✅ Comprehensive health monitoring system
- ✅ Test endpoints for demonstrating fault tolerance
- ✅ Production-ready configuration and monitoring
- ✅ Self-healing capabilities for transient failures

**Access the system**: http://192.168.0.138:7878/ (Login: admin/admin)
**Test endpoints**: Use `/api/test/circuit-breaker/{service}` to demonstrate fault tolerance
**Health monitoring**: Check `/health/detailed` for comprehensive system status