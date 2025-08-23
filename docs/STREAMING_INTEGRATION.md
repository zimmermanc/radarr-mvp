# Streaming Service Integration Documentation

## Overview

The Radarr MVP includes a comprehensive streaming service integration that aggregates data from multiple sources to provide:
- Trending movies and TV shows
- Streaming availability information
- Upcoming releases to streaming platforms
- Provider filtering and search

## Architecture

### Data Sources

1. **TMDB (The Movie Database)**
   - Primary source for trending content
   - Watch provider information
   - Movie and TV show metadata
   - Rate limit: Standard API limits
   - Cache TTL: 3-24 hours

2. **Trakt**
   - Additional trending data with social metrics
   - User engagement scores
   - OAuth 2.0 device flow authentication
   - Token expiry: 24 hours (auto-refresh required)
   - Cache TTL: 1 hour

3. **Watchmode**
   - Streaming availability data
   - Deep links to streaming services
   - CSV-based ID mapping (TMDB ↔ Watchmode)
   - Rate limit: 1000 requests/month (~33/day)
   - Cache TTL: 12 hours

### Database Schema

The integration uses 6 PostgreSQL tables:

```sql
-- Cache for API responses
streaming_cache (
  cache_key: TEXT PRIMARY KEY
  data: JSONB
  expires_at: TIMESTAMPTZ
)

-- ID mappings between services
streaming_id_mappings (
  tmdb_id: INTEGER
  watchmode_id: TEXT
  media_type: TEXT
  last_verified: TIMESTAMPTZ
)

-- Trending content
trending_entries (
  id: UUID PRIMARY KEY
  tmdb_id: INTEGER
  media_type: TEXT
  title: TEXT
  source: TEXT
  time_window: TEXT
  rank: INTEGER
  score: DECIMAL
  -- ... additional metadata
)

-- Streaming availability
streaming_availability (
  id: UUID PRIMARY KEY
  tmdb_id: INTEGER
  service_name: TEXT
  service_type: TEXT
  deep_link: TEXT
  price_amount: DECIMAL
  -- ... additional fields
)

-- OAuth tokens
oauth_tokens (
  service: TEXT PRIMARY KEY
  access_token: TEXT
  refresh_token: TEXT
  expires_at: TIMESTAMPTZ
)

-- Coming soon releases
coming_soon_releases (
  id: UUID PRIMARY KEY
  tmdb_id: INTEGER
  title: TEXT
  release_date: DATE
  streaming_services: JSONB
)
```

## API Endpoints

### GET /api/streaming/trending/:media_type

Get trending movies or TV shows.

**Parameters:**
- `media_type`: `movie` or `tv` (required)
- `window`: `day` or `week` (optional, default: `day`)
- `source`: `tmdb`, `trakt`, or `aggregated` (optional, default: `aggregated`)
- `limit`: Number of results (optional, default: 20)

**Response:**
```json
{
  "data": {
    "entries": [
      {
        "tmdb_id": 603,
        "title": "The Matrix",
        "rank": 1,
        "score": 95.5,
        "vote_average": 8.2,
        "release_date": "1999-03-30",
        "poster_path": "/path/to/poster.jpg"
      }
    ],
    "window": "day",
    "source": "aggregated",
    "fetched_at": "2025-08-23T12:00:00Z"
  }
}
```

### GET /api/streaming/availability/:tmdb_id

Get streaming availability for a specific movie or TV show.

**Parameters:**
- `tmdb_id`: TMDB ID of the content (required)
- `region`: ISO country code (optional, default: `US`)

**Response:**
```json
{
  "data": {
    "tmdb_id": 603,
    "media_type": "movie",
    "region": "US",
    "items": [
      {
        "service_name": "Netflix",
        "service_type": "subscription",
        "deep_link": "https://netflix.com/watch/...",
        "quality": "HD"
      },
      {
        "service_name": "Amazon Prime Video",
        "service_type": "rent",
        "price_amount": 3.99,
        "price_currency": "USD",
        "quality": "4K"
      }
    ],
    "fetched_at": "2025-08-23T12:00:00Z"
  }
}
```

### GET /api/streaming/coming-soon/:media_type

Get upcoming releases to streaming platforms.

**Parameters:**
- `media_type`: `movie` or `tv` (required)
- `region`: ISO country code (optional, default: `US`)

**Response:**
```json
{
  "data": {
    "media_type": "movie",
    "region": "US",
    "releases": [
      {
        "tmdb_id": 12345,
        "title": "Upcoming Movie",
        "release_date": "2025-09-01",
        "streaming_services": ["Netflix", "Hulu"],
        "poster_path": "/path/to/poster.jpg"
      }
    ],
    "fetched_at": "2025-08-23T12:00:00Z"
  }
}
```

### GET /api/streaming/providers

Get list of available streaming providers.

**Parameters:**
- `region`: ISO country code (optional, default: `US`)

**Response:**
```json
{
  "data": {
    "providers": [
      {
        "name": "Netflix",
        "logo_url": "https://...",
        "service_types": ["subscription"],
        "regions": ["US", "CA", "UK"]
      }
    ],
    "region": "US"
  }
}
```

### POST /api/streaming/cache/refresh

Force refresh of cached data.

**Response:**
```json
{
  "message": "Cache refresh initiated"
}
```

### POST /api/streaming/trakt/auth/init

Initialize Trakt OAuth device flow.

**Response:**
```json
{
  "data": {
    "device_code": "abc123...",
    "user_code": "DEF456",
    "verification_url": "https://trakt.tv/activate",
    "expires_in": 600,
    "interval": 5
  }
}
```

## Configuration

### Environment Variables

```bash
# TMDB Configuration (Required)
TMDB_API_KEY=your_tmdb_api_key

# Trakt Configuration (Optional)
TRAKT_CLIENT_ID=your_trakt_client_id
TRAKT_CLIENT_SECRET=your_trakt_client_secret

# Watchmode Configuration (Optional)
WATCHMODE_API_KEY=your_watchmode_api_key

# Database (Required)
DATABASE_URL=postgresql://user:pass@localhost/radarr
```

### Cache TTL Configuration

The system uses aggressive caching to respect API rate limits:

| Service | Endpoint | TTL (hours) | Reason |
|---------|----------|-------------|---------|
| TMDB | Trending | 3 | Frequently updated |
| TMDB | Providers | 24 | Rarely changes |
| Trakt | Trending | 1 | Real-time social data |
| Watchmode | Availability | 12 | Rate limit constraints |
| Aggregated | Trending | 1 | Combined freshness |
| Coming Soon | All | 24 | Daily updates sufficient |

## React UI Components

### TrendingCarousel

Displays trending movies/TV shows in a horizontal carousel.

```tsx
import { TrendingCarousel } from '@/components/streaming';

<TrendingCarousel
  mediaType="movie"
  timeWindow="day"
  onMovieSelect={(tmdbId) => console.log(tmdbId)}
/>
```

### StreamingAvailability

Shows where content is available for streaming.

```tsx
import { StreamingAvailability } from '@/components/streaming';

<StreamingAvailability
  tmdbId={603}
  region="US"
  compact={false}
/>
```

### ComingSoonList

Lists upcoming streaming releases.

```tsx
import { ComingSoonList } from '@/components/streaming';

<ComingSoonList
  mediaType="movie"
  region="US"
  limit={10}
  onMovieSelect={(tmdbId) => console.log(tmdbId)}
/>
```

### ProviderFilter

Filter content by streaming provider.

```tsx
import { ProviderFilter } from '@/components/streaming';

<ProviderFilter
  region="US"
  selectedProviders={['Netflix', 'Hulu']}
  onProvidersChange={(providers) => console.log(providers)}
/>
```

## Testing

### Running Tests

```bash
# Run all streaming integration tests
cargo test streaming

# Run specific test
cargo test test_trending_movies_endpoint

# Run with output
cargo test streaming -- --nocapture
```

### Manual Testing

```bash
# Test trending endpoint
curl -H "X-Api-Key: your_api_key" \
  "http://localhost:7878/api/streaming/trending/movie?window=day"

# Test availability
curl -H "X-Api-Key: your_api_key" \
  "http://localhost:7878/api/streaming/availability/603?region=US"

# Initialize Trakt OAuth
curl -X POST -H "X-Api-Key: your_api_key" \
  "http://localhost:7878/api/streaming/trakt/auth/init"
```

## Performance Considerations

### Query Optimization

- All database queries use indexes on `tmdb_id`, `media_type`, and `expires_at`
- JSONB data is indexed with GIN for fast lookups
- Batch operations used for bulk inserts

### Caching Strategy

1. **Two-tier caching**: Database cache + optional Redis layer
2. **Automatic expiry**: Background job cleans expired entries
3. **Smart invalidation**: Only refresh when TTL expired
4. **Fallback handling**: Return stale data if API fails

### Rate Limit Management

- **Watchmode**: Maximum 33 requests/day enforced
- **TMDB**: Standard rate limiting with exponential backoff
- **Trakt**: OAuth token refresh before expiry

## Troubleshooting

### Common Issues

1. **No streaming data appearing**
   - Check API keys are configured
   - Verify database migrations ran successfully
   - Check cache expiry times

2. **Trakt authentication failing**
   - Ensure OAuth credentials are valid
   - Check token expiry (24 hours)
   - Verify device code hasn't expired (10 minutes)

3. **Slow response times**
   - Check database indexes exist
   - Monitor cache hit rates
   - Consider increasing cache TTLs

### Debug Logging

Enable debug logging for streaming components:

```bash
RUST_LOG=radarr_infrastructure::streaming=debug cargo run
```

## Future Enhancements

1. **Additional Providers**
   - JustWatch API integration
   - Reelgood data source
   - Regional provider support

2. **Advanced Features**
   - Price comparison across services
   - Notification when content becomes available
   - Personalized recommendations

3. **Performance**
   - Redis caching layer
   - GraphQL API for efficient queries
   - WebSocket for real-time updates

## License and Attribution

- TMDB: This product uses the TMDB API but is not endorsed or certified by TMDB
- Trakt: OAuth implementation follows Trakt API v2 guidelines
- Watchmode: Uses public CSV mapping data under fair use