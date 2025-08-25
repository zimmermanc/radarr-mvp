# Streaming Service Setup Guide

## Quick Start

### 1. Obtain API Keys

#### TMDB (Required)
1. Visit https://www.themoviedb.org/settings/api
2. Create an account if needed
3. Request an API key (choose "Developer" for personal use)
4. Copy your API Key (v3 auth)

#### Trakt (Optional - Enhanced trending data)
1. Visit https://trakt.tv/oauth/applications
2. Create a new application
3. Set redirect URI to `urn:ietf:wg:oauth:2.0:oob`
4. Copy Client ID and Client Secret

#### Watchmode (Optional - Streaming availability)
1. Visit https://api.watchmode.com/
2. Sign up for free tier (1000 requests/month)
3. Copy your API key from dashboard

### 2. Configure Environment

Add to your `.env` file:

```bash
# Required
TMDB_API_KEY=eyJhbGciOiJIUzI1NiJ9...your_key_here

# Optional
TRAKT_CLIENT_ID=abc123...
TRAKT_CLIENT_SECRET=def456...
WATCHMODE_API_KEY=ghi789...
```

### 3. Run Database Migrations

```bash
# Apply streaming tables migration
export DATABASE_URL="postgresql://radarr:radarr@localhost/radarr"
psql -U radarr -d radarr -f migrations/004_streaming_integration.sql
```

### 4. Start the Server

```bash
cargo run --release
```

### 5. Verify Installation

```bash
# Check health endpoint
curl http://localhost:7878/health

# Test trending endpoint
curl -H "X-Api-Key: YOUR_API_KEY_HERE" \
  http://localhost:7878/api/streaming/trending/movie
```

## Frontend Setup

### 1. Install Dependencies

```bash
cd web
npm install
```

### 2. Configure API Connection

Create/update `web/.env`:

```bash
VITE_API_URL=http://localhost:7878
VITE_API_KEY=YOUR_API_KEY_HERE
```

### 3. Start Development Server

```bash
npm run dev
```

### 4. Access Streaming Features

Navigate to http://localhost:5173/streaming

## Trakt OAuth Setup

### Device Flow Authentication

1. Initialize authentication:
```bash
curl -X POST -H "X-Api-Key: your_api_key" \
  http://localhost:7878/api/streaming/trakt/auth/init
```

2. Response will include:
```json
{
  "device_code": "abc123...",
  "user_code": "DEF456",
  "verification_url": "https://trakt.tv/activate"
}
```

3. Direct user to visit the URL and enter the code
4. System will automatically poll for completion
5. Token stored in database with 24-hour expiry

### Token Refresh

Tokens are automatically refreshed before expiry. Manual refresh:

```bash
# Force token refresh (if implemented)
curl -X POST -H "X-Api-Key: your_api_key" \
  http://localhost:7878/api/streaming/trakt/auth/refresh
```

## Cache Management

### Understanding Cache Behavior

The system caches API responses to:
1. Reduce API calls (respect rate limits)
2. Improve response times
3. Provide offline capability

### Cache TTL Defaults

| Data Type | TTL | Rationale |
|-----------|-----|-----------|
| TMDB Trending | 3 hours | Balance freshness vs API calls |
| TMDB Providers | 24 hours | Rarely changes |
| Trakt Trending | 1 hour | Real-time social metrics |
| Watchmode Data | 12 hours | Strict rate limits (33/day) |
| Coming Soon | 24 hours | Daily updates sufficient |

### Manual Cache Management

```bash
# Force cache refresh
curl -X POST -H "X-Api-Key: your_api_key" \
  http://localhost:7878/api/streaming/cache/refresh

# View cache statistics (if implemented)
psql -d radarr -c "
  SELECT 
    cache_key,
    expires_at,
    CASE 
      WHEN expires_at > NOW() THEN 'Valid'
      ELSE 'Expired'
    END as status
  FROM streaming_cache
  ORDER BY expires_at DESC
  LIMIT 10;
"

# Clear expired cache entries
psql -d radarr -c "DELETE FROM streaming_cache WHERE expires_at < NOW();"
```

## Performance Tuning

### Database Indexes

Ensure indexes are created:

```sql
-- Verify indexes exist
\d streaming_cache
\d trending_entries
\d streaming_availability
```

### Connection Pooling

Configure in `.env`:

```bash
DATABASE_MAX_CONNECTIONS=20
DATABASE_CONNECT_TIMEOUT=30
```

### API Rate Limiting

Monitor API usage:

```bash
# Check Watchmode usage (limited to 1000/month)
psql -d radarr -c "
  SELECT 
    DATE(fetched_at) as date,
    COUNT(*) as api_calls
  FROM streaming_cache
  WHERE cache_key LIKE 'watchmode:%'
    AND fetched_at > NOW() - INTERVAL '30 days'
  GROUP BY DATE(fetched_at)
  ORDER BY date DESC;
"
```

## Troubleshooting

### No Data Appearing

1. **Check API Keys**
```bash
# Verify environment variables
echo $TMDB_API_KEY
echo $TRAKT_CLIENT_ID
echo $WATCHMODE_API_KEY
```

2. **Test API Directly**
```bash
# Test TMDB
curl "https://api.themoviedb.org/3/movie/popular?api_key=$TMDB_API_KEY"

# Test Watchmode (be careful - counts against quota!)
curl "https://api.watchmode.com/v1/status/?apiKey=$WATCHMODE_API_KEY"
```

3. **Check Database**
```bash
# Verify tables exist
psql -d radarr -c "\dt streaming*"

# Check for cached data
psql -d radarr -c "SELECT COUNT(*) FROM streaming_cache;"
```

### Slow Performance

1. **Check Cache Hit Rate**
```bash
# Monitor cache effectiveness
psql -d radarr -c "
  SELECT 
    CASE 
      WHEN expires_at > NOW() THEN 'Hit'
      ELSE 'Miss'
    END as cache_status,
    COUNT(*) as count
  FROM streaming_cache
  GROUP BY cache_status;
"
```

2. **Increase Cache TTL**
Modify `aggregator_factory.rs`:
```rust
cache_ttl.insert("tmdb_trending".to_string(), 6);  // Increase to 6 hours
```

### API Errors

1. **Enable Debug Logging**
```bash
RUST_LOG=debug cargo run
```

2. **Check Response Codes**
- 401: Invalid API key
- 429: Rate limit exceeded
- 500: Service error

## Monitoring

### Health Checks

```bash
# Create monitoring script
cat > check_streaming.sh << 'EOF'
#!/bin/bash

# Check trending endpoint
TRENDING=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "X-Api-Key: $API_KEY" \
  "http://localhost:7878/api/streaming/trending/movie")

# Check cache size
CACHE_SIZE=$(psql -t -d radarr -c "SELECT COUNT(*) FROM streaming_cache;")

echo "Trending Status: $TRENDING"
echo "Cache Entries: $CACHE_SIZE"

if [ "$TRENDING" != "200" ]; then
  echo "WARNING: Streaming endpoint not responding"
  exit 1
fi
EOF

chmod +x check_streaming.sh
```

### Metrics to Track

1. **API Usage**
   - Calls per day to each service
   - Cache hit/miss ratio
   - Average response time

2. **Data Freshness**
   - Age of cached entries
   - Successful vs failed refreshes
   - Token expiry warnings

3. **User Engagement**
   - Most viewed trending items
   - Click-through to streaming services
   - Popular providers by region

## Best Practices

1. **API Key Security**
   - Never commit keys to git
   - Use environment variables
   - Rotate keys periodically

2. **Cache Strategy**
   - Start with conservative TTLs
   - Monitor and adjust based on usage
   - Implement cache warming for popular queries

3. **Error Handling**
   - Always provide fallbacks
   - Log errors for debugging
   - Show user-friendly messages

4. **Testing**
   - Test with expired cache
   - Simulate API failures
   - Verify rate limit handling

## Support

For issues or questions:
1. Check logs: `RUST_LOG=debug cargo run`
2. Review documentation: `/docs/STREAMING_INTEGRATION.md`
3. Database diagnostics: `psql -d radarr`
4. API testing: Use Postman or curl commands above