# Circuit Breaker and Health Check Implementation

## Overview

This implementation adds comprehensive circuit breakers and health monitoring to the Radarr MVP, protecting against external service failures and providing detailed system health information.

## Features Implemented

### 1. Generic Circuit Breaker (`radarr-core/src/circuit_breaker.rs`)

- **Three States**:
  - **Closed**: Normal operation, requests pass through
  - **Open**: Service is failing, requests are rejected immediately  
  - **Half-Open**: Testing recovery, single request allowed through

- **Configurable Parameters**:
  - Failure threshold (default: 5 consecutive failures)
  - Timeout duration (default: 30 seconds)
  - Request timeout (default: 10 seconds)
  - Success threshold for closing (default: 1 success)

- **Advanced Features**:
  - Atomic counters for lock-free metrics
  - Comprehensive metrics tracking (success rate, response times)
  - Health assessment based on success rate and failure patterns
  - Manual circuit control for testing and debugging

### 2. Service Integration

#### TMDB Client (`radarr-infrastructure/src/tmdb/client.rs`)
- Circuit breaker with 5 failure threshold
- 30-second recovery timeout
- 10-second request timeout
- 2 successes required for recovery

#### HDBits Client (`radarr-indexers/src/hdbits/client.rs`)
- Lower failure threshold (3) due to scraping sensitivity
- 60-second recovery timeout for stability
- Rate limiting preserved outside circuit breaker
- Session authentication protection

#### qBittorrent Client (`radarr-downloaders/src/qbittorrent.rs`)
- Standard 3 failure threshold
- 30-second recovery timeout
- Connection test and authentication protection
- Cookie-based session management preserved

### 3. Enhanced Health Check System (`radarr-api/src/handlers/health.rs`)

#### Available Endpoints

- **GET /health** - Basic health check with uptime
- **GET /health/detailed** - Comprehensive system status
- **GET /health/services/{service}** - Individual service status

#### Service Monitoring

Each service reports:
- Current circuit breaker state
- Response time metrics
- Last success/failure timestamps
- Error details when unhealthy
- Success rate calculations

#### Supported Services

- **TMDB**: Movie metadata service
- **HDBits**: Torrent indexer service  
- **qBittorrent**: Download client service
- **PostgreSQL**: Database connection pool
- **Queue Processor**: Background task system

### 4. Production-Ready Features

#### Error Handling
- Graceful degradation during outages
- Meaningful error messages with context
- Proper error categorization (timeout, authentication, etc.)
- Circuit breaker state in error responses

#### Monitoring
- Real-time metrics collection
- Service health assessment
- Connection pool utilization tracking
- Queue processor status monitoring

#### Logging
- Structured logging with tracing
- Circuit breaker state transitions
- Service recovery notifications
- Performance metrics logging

## Circuit Breaker Behavior

### State Transitions

```
CLOSED --[5 failures]--> OPEN --[30s timeout]--> HALF_OPEN
   ^                                                  |
   +----------------[1 success]---------------------+
                                                     |
                                          [failure] |
                                                     v
                                                   OPEN
```

### Failure Scenarios

1. **Network Timeouts**: Request exceeds configured timeout
2. **HTTP Errors**: 4xx/5xx status codes from external services
3. **Authentication Failures**: Invalid credentials or expired sessions
4. **Service Unavailable**: External service completely down

### Recovery Behavior

1. **Immediate**: Circuit remains closed for intermittent failures
2. **Gradual**: Half-open state tests recovery before full restoration
3. **Progressive**: Success rate must be >80% for healthy status
4. **Monitored**: All state changes are logged and tracked

## Implementation Benefits

### Reliability
- Prevents cascading failures
- Fast failure detection and response
- Automatic recovery without manual intervention
- Graceful degradation during outages

### Observability
- Real-time service health monitoring
- Detailed metrics for troubleshooting
- Historical success/failure patterns
- Performance trend analysis

### Production Readiness
- Battle-tested failure thresholds
- Configurable parameters for different services
- Comprehensive error handling
- Structured logging for operations

## Usage Examples

### Health Check Response

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "services": [
    {
      "name": "TMDB",
      "status": "healthy",
      "response_time_ms": 150,
      "last_check": "2024-01-15T12:30:45Z",
      "error": null
    },
    {
      "name": "HDBits", 
      "status": "degraded",
      "response_time_ms": 2000,
      "last_check": "2024-01-15T12:30:45Z",
      "error": "Circuit breaker testing recovery"
    }
  ]
}
```

### Service-Specific Health

```bash
curl http://localhost:7878/health/services/tmdb
curl http://localhost:7878/health/services/hdbits
curl http://localhost:7878/health/services/qbittorrent
curl http://localhost:7878/health/services/database
curl http://localhost:7878/health/services/queue
```

## Configuration

Circuit breaker settings can be customized per service:

```rust
let config = CircuitBreakerConfig::new("ServiceName")
    .with_failure_threshold(3)
    .with_timeout(Duration::from_secs(60))
    .with_request_timeout(Duration::from_secs(15))
    .with_success_threshold(2);
```

## Testing

The circuit breaker implementation includes comprehensive tests for:
- State transitions (closed → open → half-open → closed)
- Timeout handling and recovery
- Metrics accuracy and thread safety
- Manual circuit control
- Error handling edge cases

## Performance Impact

- **Minimal Overhead**: Atomic operations for metrics
- **No Blocking**: Async-first design with tokio
- **Memory Efficient**: Shared circuit breaker instances
- **Lock-Free Metrics**: High-performance counters
- **Fast Failure**: Immediate rejection when circuit is open

This implementation provides enterprise-grade resilience and observability for the Radarr MVP, ensuring reliable operation even when external services experience issues.