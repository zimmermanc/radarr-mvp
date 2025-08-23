# HDBits Scene Group Analyzer

## Overview

The HDBits Scene Group Analyzer is a comprehensive system for collecting and analyzing torrent release data from HDBits to build quality scoring metrics for scene groups. This data is used by the Radarr decision engine to make intelligent choices about which releases to download.

## Architecture

### Authentication Methods

1. **API Authentication** (Production Indexer)
   - Uses username + passkey for API access
   - Located in `HDBITS_USERNAME` and `HDBITS_PASSKEY` environment variables
   - Used for automated searching and release selection

2. **Session Authentication** (Analysis Tools)
   - Uses browser session cookies for browse.php access
   - Located in `HDBITS_SESSION_COOKIE` environment variable
   - Fallback method if API is unavailable

## API Endpoint Details

### Base URL
```
https://hdbits.org/api/torrents
```

### Authentication
- **Method**: POST with JSON body
- **Required fields**: `username` and `passkey`
- **2FA Note**: API usage requires active 2FA on the account (since 2023-06-22)

### Request Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `category` | int[] | 1=Movie, 2=TV, 3=Documentary, 4=Music, 5=Sport |
| `codec` | int[] | 1=H.264, 2=MPEG-2, 3=VC-1, 4=XviD, 5=HEVC |
| `medium` | int[] | 1=Blu-ray, 3=Encode, 4=Capture, 5=Remux, 6=WEB-DL |
| `origin` | int[] | 0=Scene/Undefined, 1=Internal |
| `exclusive` | int[] | 0=Non-exclusive, 1=Exclusive |
| `limit` | int | Max results per page (1-100, default 30) |
| `page` | int | Page number (0-based) |

### Response Fields

Key fields we extract:
- `id`: Torrent ID
- `name`: Release name
- `size`: Size in bytes
- `seeders`: Current seeders
- `times_completed`: Times snatched
- `type_exclusive`: 1 if exclusive
- `type_origin`: 1 if internal
- `type_codec`: Codec ID
- `type_medium`: Medium ID
- `utadded`: Unix timestamp when added

## Quality Scoring Algorithm

### Score Components

1. **Base Score** (0-50 points)
   - Based on release count
   - Capped at 50 to prevent single groups from dominating

2. **Exclusive Bonus** (+50 points per release)
   - Highest quality indicator
   - Internal HDBits releases marked as exclusive

3. **Internal Bonus** (+30 points per release)
   - High quality indicator
   - Internal but not necessarily exclusive

4. **Health Score** (0-20 points)
   - Based on average seeders
   - Indicates content availability

5. **Popularity Score** (0-10 points)
   - Based on average times snatched
   - Indicates community trust

6. **Codec Bonuses**
   - HEVC/H.265: +15 points
   - H.264: +10 points

7. **Medium Bonuses**
   - Blu-ray: +20 points
   - Remux: +15 points
   - WEB-DL: +10 points

### Top Scene Groups Identified

From our analysis of 2000 releases:

| Rank | Group | Score | Releases | Type |
|------|-------|-------|----------|------|
| 1 | EXCLUSIVE | 87210 | 1097 | Internal Exclusive |
| 2 | INTERNAL | 2535 | 81 | Internal Non-Exclusive |
| 3 | RAY | 117 | 40 | Scene |
| 4 | TMT | 117 | 21 | Scene |
| 5 | FRAMESTOR | 116 | 23 | Scene |
| 6 | FLUX | 115 | 38 | Scene |
| 7 | BYNDR | 115 | 55 | Scene |
| 8 | CINEPHILES | 110 | 20 | Scene |
| 9 | B0MBARDIERS | 107 | 23 | Scene |
| 10 | KHN | 100 | 52 | Scene |

## Data Collection Strategy

### Filters Used

**Pass 1: Exclusive/Internal Releases**
- Category: Movies only
- Exclusive: Yes
- Origin: Internal
- All codecs and mediums accepted

**Pass 2: High-Quality Scene Releases**
- Category: Movies only
- Codecs: H.264, HEVC (excluding XviD, MPEG-2)
- Mediums: Blu-ray, Encode, Remux, WEB-DL (excluding Capture)
- Any origin/exclusivity

### Rate Limiting

- **Recommended**: 10 seconds between requests
- **Aggressive**: 5 seconds between requests
- **Conservative**: 30 seconds between requests
- **Ultra-safe**: 60 seconds between requests

### Data Volume Estimates

**1 Year of Data:**
- ~22,400 torrents
- ~224 API requests
- ~37 minutes at 10 sec/request

**2 Years of Data:**
- ~45,000 torrents
- ~450 API requests
- ~75 minutes at 10 sec/request

## Implementation Files

### Core Analyzer
- `/crates/analysis/src/hdbits_api_analyzer.rs` - Rust implementation
- `/tmp/radarr/analysis/hdbits_api_analyzer.py` - Python implementation

### Supporting Tools
- `/crates/analysis/src/bin/hdbits-api-analyzer.rs` - CLI binary
- `/tmp/radarr/analysis/test_api_correct.py` - API testing tool
- `/tmp/radarr/analysis/import_to_postgres.py` - Database import

### Configuration
- `config/services.env` - Contains credentials
  - `HDBITS_USERNAME` - API username
  - `HDBITS_PASSKEY` - API passkey (32-64 chars)
  - `HDBITS_SESSION_COOKIE` - Session cookie (fallback)

## Database Schema

### Tables Used

**scene_groups**
- `id`: UUID primary key
- `name`: Group name (unique)
- `group_type`: 'scene' or 'internal'
- `reputation_score`: Calculated quality score
- `last_seen`: Timestamp
- `is_trusted`: Boolean

**scene_group_metrics**
- `scene_group_id`: Foreign key
- `analysis_date`: Date of analysis
- `release_count`: Number of releases
- `total_size_gb`: Total size
- `average_seeders`: Avg seeders
- `average_leechers`: Avg leechers
- `resolution_distribution`: JSONB

**analysis_sessions**
- `id`: UUID primary key
- `session_type`: 'comprehensive' or 'incremental'
- `source`: 'hdbits'
- `started_at`: Timestamp
- `completed_at`: Timestamp
- `status`: 'running', 'completed', 'failed'
- `releases_analyzed`: Count
- `groups_discovered`: Count

## Usage Examples

### Run 2-Year Analysis
```bash
# Using Python implementation
cd /tmp/radarr/analysis
python3 hdbits_2year_analyzer.py

# Using Rust implementation (when fixed)
cargo run --bin hdbits-api-analyzer -- \
  --output /tmp/radarr/analysis \
  --max-pages 450 \
  --rate-limit-seconds 10
```

### Import to PostgreSQL
```bash
python3 import_to_postgres.py api_analysis_TIMESTAMP.json --show-top 20
```

### Test API Connection
```bash
python3 test_api_correct.py
```

## Security Considerations

1. **Never commit credentials** to version control
2. **Use environment variables** for sensitive data
3. **Respect rate limits** to avoid API bans
4. **Store passkey securely** - it's equivalent to password
5. **Rotate credentials** periodically

## Monitoring

### Success Indicators
- Consistent quality scores across runs
- High percentage of releases with identified groups
- Low number of "UNKNOWN" group classifications

### Failure Indicators
- Authentication errors (status code 5)
- Rate limit errors
- Declining data quality
- Increasing "UNKNOWN" classifications

## Future Enhancements

1. **Incremental Updates**: Collect only new releases since last run
2. **TV Show Support**: Extend to category 2 (TV)
3. **Enhanced Patterns**: Better scene group extraction from release names
4. **Real-time Scoring**: Update scores as new releases arrive
5. **Machine Learning**: Use ML to predict release quality
6. **Cross-Tracker Analysis**: Compare with other trackers

## Troubleshooting

### Common Issues

**Authentication Failed (Status 5)**
- Verify passkey is correct (not the session cookie password)
- Ensure 2FA is enabled on the account
- Check username spelling

**JSON Malformed (Status 3)**
- Ensure using POST method with JSON body
- Check Content-Type header is application/json
- Verify JSON structure

**No Data Returned**
- Check filters aren't too restrictive
- Verify category IDs are correct
- Ensure page number is valid

## References

- [HDBits API Documentation](https://hdbits.org/wiki/index.php?title=API)
- [Scene Release Naming Standards](https://scenerules.org/)
- [Radarr Quality Definitions](https://wiki.servarr.com/radarr/settings#quality-definitions)