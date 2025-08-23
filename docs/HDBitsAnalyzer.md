# HDBits Quality Analysis Service Documentation

## Table of Contents
1. [Overview](#overview)
2. [Quality Standards Research](#quality-standards-research)
3. [Service Architecture](#service-architecture)
4. [Database Schema](#database-schema)
5. [Quality Scoring Algorithm](#quality-scoring-algorithm)
6. [API Endpoints](#api-endpoints)
7. [Data Collection Strategy](#data-collection-strategy)
8. [Analysis Scripts](#analysis-scripts)
9. [Deployment Guide](#deployment-guide)
10. [Monitoring & Maintenance](#monitoring--maintenance)

## Overview

The HDBits Quality Analysis Service is an integrated Rust module within Radarr MVP that continuously evaluates torrent releases to provide quality scores and recommendations. It combines historical analysis of 2+ years of data with real-time monitoring of new releases.

### Key Features
- Continuous 24/7 data collection and analysis
- Resilient checkpoint/resume system
- Quality scoring based on extensive research
- Scene group reputation tracking
- API endpoints for quality recommendations
- Integration with Radarr MVP search results

### Current Status (as of 2025-08-23)
- **Data Collected**: 29,225 torrents
- **Scene Groups Identified**: 1,519
- **Collection Runtime**: 59 minutes (ongoing)
- **Analysis Coverage**: 2 years of releases

## Quality Standards Research

### What Makes a "Quality" Movie Encode

Based on research from AVSForum, Doom9, and elite tracker communities, we've identified the following quality markers:

#### Video Excellence Criteria
1. **Transparent Quality CRF Values**
   - CRF 17-19: Visually lossless (transparent)
   - CRF 20-22: Near-transparent with file size benefits
   - CRF <17: Diminishing returns, massive file sizes

2. **Codec Standards**
   - **Preferred**: x265/HEVC 10-bit (efficiency + HDR support)
   - **Acceptable**: x264/AVC High Profile
   - **Avoid**: XviD, MPEG-2, VC-1 (outdated)

3. **Encoding Settings**
   ```
   x265 optimal: --preset slow --crf 18 --no-sao
   x265 HDR: --hdr-opt --master-display --max-cll
   Grain retention: --tune grain (for grainy content)
   ```

4. **HDR Requirements**
   - HDR10/HDR10+ metadata preservation
   - Dolby Vision layer retention when available
   - 10-bit color depth mandatory for HDR
   - Wide color gamut (BT.2020) preservation

#### Audio Excellence Criteria
1. **Lossless Audio Mandatory for Quality**
   - DTS-HD Master Audio
   - Dolby TrueHD
   - FLAC
   - PCM

2. **Object-Based Audio Bonus**
   - Dolby Atmos
   - DTS:X
   - Properly flagged and functional

3. **Multiple Track Requirements**
   - Original language track
   - Commentary tracks preserved
   - Foreign language options

### P2P vs Scene Philosophy

#### Scene Groups (Speed-Focused)
- **Priority**: First to release
- **Constraints**: Strict size rules (4-15GB typical)
- **Quality**: Consistent but often compromised
- **Examples**: SPARKS, AMIABLE, GECKOS

#### P2P Groups (Quality-Focused)
- **Priority**: Optimal quality
- **Constraints**: None - quality over size
- **Quality**: Transparent encoding goal
- **Elite Groups**: DON, CtrlHD, ESiR, CHD, FraMeSToR, HiFi

#### HDBits Exclusive/Internal Superiority
1. **Direct source access** (BD/UHD discs)
2. **Unlimited encoding time** (days per release)
3. **Multiple QC passes** before release
4. **No file size compromises**
5. **Elite encoder expertise**

## Service Architecture

### Dual Implementation Architecture

1. **Production Indexer** (`crates/indexers/src/hdbits/`)
   - Purpose: Real-time torrent searching for automated downloads
   - Authentication: API-based using username + passkey
   - Integration: Part of main application workflow
   - Use Case: Finding and downloading releases

2. **Analysis System** (`crates/analysis/src/`)
   - Purpose: Scene group reputation analysis
   - Authentication: Session cookie for browse.php access
   - Integration: Standalone research tools
   - Use Case: Building quality scoring database

### System Components

```
radarr-mvp/
├── src/
│   ├── analysis/
│   │   ├── mod.rs                    # Module entry point
│   │   ├── hdbits_collector.rs       # Continuous data collector
│   │   ├── quality_scorer.rs         # Scoring algorithm implementation
│   │   ├── scene_analyzer.rs         # Scene group analysis
│   │   ├── analysis_service.rs       # Background service coordinator
│   │   └── deep_analyzer.rs          # Details.php scraper
│   ├── models/
│   │   ├── scene_group.rs            # Scene group model
│   │   ├── torrent_analysis.rs       # Analysis result model
│   │   └── quality_metrics.rs        # Quality metrics model
│   └── api/
│       └── analysis_endpoints.rs     # REST API endpoints
├── migrations/
│   └── 004_analysis_tables.sql       # Database schema
└── config/
    └── analysis.toml                  # Configuration
```

### Core Service Implementation

```rust
// src/analysis/analysis_service.rs
pub struct AnalysisService {
    db: Arc<PgPool>,
    hdbits_client: HDBitsClient,
    config: AnalysisConfig,
    running: Arc<AtomicBool>,
}

impl AnalysisService {
    pub async fn start(self: Arc<Self>) {
        // Spawn background tasks
        tokio::spawn(self.clone().run_historical_backfill());
        tokio::spawn(self.clone().run_incremental_updates());
        tokio::spawn(self.clone().run_scoring_updates());
        tokio::spawn(self.clone().run_deep_analysis());
    }
    
    async fn run_historical_backfill(&self) {
        // Collect 2 years of historical data
        // Resume from checkpoint if available
    }
    
    async fn run_incremental_updates(&self) {
        // Check for new releases every 5 minutes
        // Process and score immediately
    }
    
    async fn run_deep_analysis(&self) {
        // Fetch details.php for high-value torrents
        // Extract MediaInfo and technical specs
    }
}
```

## Database Schema

```sql
-- Scene groups with reputation tracking
CREATE TABLE scene_groups (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    quality_score DECIMAL(6,2) DEFAULT 0,
    reputation_tier VARCHAR(20), -- legendary/elite/trusted/established/developing/newcomer
    total_releases INTEGER DEFAULT 0,
    exclusive_releases INTEGER DEFAULT 0,
    internal_releases INTEGER DEFAULT 0,
    avg_seeders DECIMAL(8,2),
    avg_snatched DECIMAL(8,2),
    specialization VARCHAR(50), -- remux_specialist/hevc_pioneer/webdl_specialist
    last_updated TIMESTAMP DEFAULT NOW(),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Analyzed torrents with quality metrics
CREATE TABLE torrent_analyses (
    id SERIAL PRIMARY KEY,
    hdbits_id INTEGER UNIQUE NOT NULL,
    name TEXT NOT NULL,
    scene_group_id INTEGER REFERENCES scene_groups(id),
    size_bytes BIGINT,
    seeders INTEGER,
    leechers INTEGER,
    times_completed INTEGER,
    upload_date TIMESTAMP,
    
    -- Quality indicators
    is_exclusive BOOLEAN DEFAULT FALSE,
    is_internal BOOLEAN DEFAULT FALSE,
    is_freeleech BOOLEAN DEFAULT FALSE,
    freeleech_percent SMALLINT,
    
    -- Technical specs
    codec VARCHAR(20),         -- H264/H265/XviD/MPEG2
    codec_profile VARCHAR(50), -- Main10/High/etc
    medium VARCHAR(20),        -- BluRay/Remux/WEB-DL/Encode
    resolution VARCHAR(10),    -- 2160p/1080p/720p
    bit_depth SMALLINT,        -- 8/10/12
    
    -- HDR metadata
    has_hdr BOOLEAN DEFAULT FALSE,
    hdr_format VARCHAR(20),    -- HDR10/HDR10+/DV/HLG
    
    -- Audio specs
    audio_formats JSONB,       -- ["DTS-HD MA", "TrueHD", "Atmos"]
    audio_languages JSONB,     -- ["English", "French", "Commentary"]
    
    -- Calculated scores
    quality_score DECIMAL(6,2),
    video_score DECIMAL(6,2),
    audio_score DECIMAL(6,2),
    source_score DECIMAL(6,2),
    
    -- Quality markers and issues
    quality_markers JSONB,     -- ["4K_resolution", "HEVC_codec", "HDR", "lossless_audio"]
    trumpable_issues JSONB,    -- ["outdated_codec", "no_hdr", "lossy_audio_only"]
    
    -- MediaInfo (from details.php)
    mediainfo_raw TEXT,
    mediainfo_parsed JSONB,
    
    -- Metadata
    analyzed_at TIMESTAMP DEFAULT NOW(),
    deep_analysis_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Collection state for resume capability
CREATE TABLE collection_state (
    id SERIAL PRIMARY KEY,
    service_name VARCHAR(50) UNIQUE NOT NULL,
    last_page INTEGER DEFAULT 0,
    last_torrent_id INTEGER,
    last_run TIMESTAMP,
    total_processed INTEGER DEFAULT 0,
    current_pass VARCHAR(50),
    state JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Quality score history for trending
CREATE TABLE quality_score_history (
    id SERIAL PRIMARY KEY,
    torrent_id INTEGER REFERENCES torrent_analyses(id),
    scene_group_id INTEGER REFERENCES scene_groups(id),
    score DECIMAL(6,2),
    score_components JSONB,
    calculated_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_scene_groups_score ON scene_groups(quality_score DESC);
CREATE INDEX idx_scene_groups_tier ON scene_groups(reputation_tier);
CREATE INDEX idx_torrents_group ON torrent_analyses(scene_group_id);
CREATE INDEX idx_torrents_date ON torrent_analyses(upload_date DESC);
CREATE INDEX idx_torrents_score ON torrent_analyses(quality_score DESC);
CREATE INDEX idx_torrents_hdbits_id ON torrent_analyses(hdbits_id);
CREATE INDEX idx_torrents_exclusive ON torrent_analyses(is_exclusive) WHERE is_exclusive = true;
CREATE INDEX idx_torrents_internal ON torrent_analyses(is_internal) WHERE is_internal = true;
```

## Quality Scoring Algorithm

### Comprehensive Scoring System (0-100 points)

```python
def calculate_quality_score(release):
    """
    Calculate quality score based on extensive research and empirical data.
    Score range: 0-100 points
    """
    score = 0.0
    
    # 1. GROUP REPUTATION (0-50 points)
    # This is the strongest predictor of quality
    if release.is_exclusive:
        score += 50  # HDBits exclusive - highest quality guarantee
    elif release.is_internal:
        score += 35  # HDBits internal - very high quality
    elif release.group in ELITE_P2P_GROUPS:
        # DON, CtrlHD, ESiR, CHD, FraMeSToR, HiFi, EbP
        score += 25
    elif release.group in TRUSTED_P2P_GROUPS:
        score += 15
    elif release.group in KNOWN_SCENE_GROUPS:
        score += 8   # Scene groups - consistent but limited by rules
    else:
        score += 0   # Unknown groups
    
    # 2. SOURCE QUALITY (0-20 points)
    if "REMUX" in release.name.upper():
        score += 20  # Untouched quality
    elif "COMPLETE" in release.name and "BLURAY" in release.name:
        score += 20  # Full disc
    elif "BluRay" in release.name or "Blu-ray" in release.name:
        score += 15  # Blu-ray encode
    elif "WEB-DL" in release.name:
        score += 12  # Untouched web source
    elif "WEBRip" in release.name:
        score += 8   # Re-encoded web source
    elif "HDTV" in release.name:
        score += 5   # Broadcast source
    
    # 3. VIDEO TECHNICAL EXCELLENCE (0-15 points)
    # Codec and bit depth
    if release.codec == "HEVC" and release.bit_depth == 10:
        score += 10  # Modern, efficient, HDR-ready
    elif release.codec == "HEVC":
        score += 8
    elif release.codec == "H264" and release.bit_depth == 10:
        score += 6
    elif release.codec == "H264":
        score += 4
    
    # Resolution bonus
    if release.resolution == "2160p":
        score += 5
    elif release.resolution == "1080p":
        score += 3
    
    # 4. HDR/COLOR SPACE (0-10 points)
    if has_dolby_vision(release):
        score += 10  # Dolby Vision - best HDR
    elif has_hdr10_plus(release):
        score += 8   # HDR10+ - dynamic metadata
    elif has_hdr10(release):
        score += 6   # HDR10 - static metadata
    elif release.bit_depth == 10:
        score += 2   # 10-bit SDR still better than 8-bit
    
    # 5. AUDIO QUALITY (0-10 points)
    audio_score = 0
    if has_lossless_audio(release):
        audio_score += 5
        if has_atmos_or_dtsx(release):
            audio_score += 5  # Object-based audio
        elif has_7_1_audio(release):
            audio_score += 3
        elif has_5_1_audio(release):
            audio_score += 2
    elif has_dts_or_dd_plus(release):
        audio_score += 3
    score += audio_score
    
    # 6. POPULARITY/HEALTH METRICS (0-5 points)
    if release.seeders > 100:
        score += 3
    elif release.seeders > 50:
        score += 2
    elif release.seeders > 20:
        score += 1
    
    if release.times_completed > 500:
        score += 2
    elif release.times_completed > 200:
        score += 1
    
    # 7. PENALTIES (negative points)
    
    # Outdated codecs
    if release.codec in ["XviD", "MPEG-2", "DivX"]:
        score -= 25  # Severely outdated
    elif release.codec == "VC-1":
        score -= 10  # Outdated but sometimes unavoidable
    
    # Missing expected features
    if release.resolution >= "1080p" and not has_lossless_audio(release):
        score -= 10  # HD should have lossless audio
    
    if release.resolution == "2160p" and not has_hdr(release):
        score -= 15  # 4K without HDR is questionable
    
    # Over-compression
    if is_likely_overcompressed(release):
        score -= 20
    
    # Size penalties for suspicious encodes
    expected_size = calculate_expected_size(release)
    if release.size < expected_size * 0.3:
        score -= 15  # Severely undersized
    elif release.size < expected_size * 0.5:
        score -= 8   # Undersized
    
    return min(100, max(0, score))

# Helper functions and group lists
ELITE_P2P_GROUPS = [
    "DON", "CtrlHD", "ESiR", "CHD", "FraMeSToR", "HiFi", "EbP",
    "TayTo", "VietHD", "BMF", "BHD", "HDMaNiAcS", "decibeL"
]

TRUSTED_P2P_GROUPS = [
    "SbR", "NCmt", "Chotab", "Geek", "V3RiTAS", "NTb", "EA",
    "CRiSC", "iFT", "TDD", "ahd", "D-Z0N3", "Dariush"
]
```

### Top Scene Groups from Analysis

| Rank | Group | Score | Releases | Type |
|------|-------|-------|----------|------|
| 1 | EXCLUSIVE | 87210 | 1097 | Internal Exclusive |
| 2 | INTERNAL | 2535 | 81 | Internal Non-Exclusive |
| 3 | Various P2P | 100-500 | Varies | Quality-focused |
| 4+ | Scene Groups | 50-150 | Varies | Speed-focused |

## API Endpoints

### HDBits API Details

**Base URL**: `https://hdbits.org/api/torrents`

**Authentication**: POST with JSON body containing `username` and `passkey`

**Request Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `category` | int[] | 1=Movie, 2=TV, 3=Documentary, 4=Music, 5=Sport |
| `codec` | int[] | 1=H.264, 2=MPEG-2, 3=VC-1, 4=XviD, 5=HEVC |
| `medium` | int[] | 1=Blu-ray, 3=Encode, 4=Capture, 5=Remux, 6=WEB-DL |
| `origin` | int[] | 0=Scene/Undefined, 1=Internal |
| `exclusive` | int[] | 0=Non-exclusive, 1=Exclusive |
| `limit` | int | Max results per page (1-100, default 30) |
| `page` | int | Page number (0-based) |

### Analysis Service Endpoints

```rust
// GET /api/analysis/groups
// Returns top scene groups by quality score

// GET /api/analysis/groups/{name}
// Returns detailed group profile

// GET /api/analysis/torrents/{id}/score
// Returns quality score breakdown

// GET /api/analysis/recommendations?min_score=75
// Returns recommended releases

// GET /api/analysis/stats
// Returns service statistics
```

## Data Collection Strategy

### Collection Phases

1. **Historical Backfill** (Initial)
   - Collect 2 years of releases
   - Use checkpoint system for resume
   - Rate limit: 10 seconds between calls
   - Time: ~75 minutes for full backfill

2. **Incremental Updates** (Continuous)
   - Check for new releases every 5 minutes
   - Process immediately for real-time scores
   - Alert on high-quality releases (score > 85)

3. **Deep Analysis** (Selective)
   - Fetch details.php for torrents with score > 80
   - Extract MediaInfo and technical specs
   - Rate limit: 10 seconds between scrapes

### Filters Used

**Pass 1: Exclusive/Internal**
- Category: Movies only
- Exclusive: Yes
- Origin: Internal

**Pass 2: High-Quality Scene**
- Category: Movies only
- Codecs: H.264, HEVC (no XviD, MPEG-2)
- Mediums: Blu-ray, Encode, Remux, WEB-DL (no Capture)

### Rate Limiting Strategy
- API calls: 10 second minimum interval
- HTML scraping: 10-15 second interval
- Exponential backoff on errors
- Circuit breaker after 5 failures

## Analysis Scripts

### Current Implementation Files

```bash
# Python implementations (temporary analysis)
/tmp/radarr/analysis/
├── hdbits_resilient_analyzer.py      # Main collector with checkpoint
├── phase2_statistical_analysis.py    # Pattern analysis
├── phase3_group_profiling.py        # Group profiles
├── phase4_deep_torrent_analysis.py  # Details.php scraping
└── *.json                           # Analysis results

# Rust implementations (production)
/home/thetu/radarr-mvp/unified-radarr/crates/analysis/src/bin/
├── hdbits-analyzer.rs               # Basic analyzer
├── hdbits-comprehensive-analyzer.rs # Full analysis
├── hdbits-session-analyzer.rs      # Session-based
└── hdbits-browse-analyzer.rs       # Browse scraper
```

### Running Analysis Pipeline

```bash
# 1. Start data collection (background)
cd /tmp/radarr/analysis
python3 hdbits_resilient_analyzer.py &

# 2. Monitor progress
tail -f resilient_analysis.log
cat analysis_progress.json

# 3. After collection, run analysis phases
python3 phase2_statistical_analysis.py
python3 phase3_group_profiling.py
python3 phase4_deep_torrent_analysis.py  # Needs session cookie

# 4. Import to database (when Rust fixed)
cargo run --bin import-analysis /tmp/radarr/analysis/2year_analysis_*.json
```

## Deployment Guide

### Prerequisites

1. **Database Setup**
   ```sql
   -- Run migrations
   psql -U radarr -d radarr < migrations/004_analysis_tables.sql
   ```

2. **Configuration** (`config/analysis.toml`)
   ```toml
   [analysis]
   enabled = true
   backfill_years = 2
   rate_limit_seconds = 10
   incremental_interval_minutes = 5
   
   [analysis.scoring]
   exclusive_bonus = 50
   internal_bonus = 35
   elite_p2p_bonus = 25
   ```

3. **Environment Variables** (`config/services.env`)
   ```env
   HDBITS_USERNAME=your_username
   HDBITS_PASSKEY=your_128_char_passkey
   HDBITS_SESSION_COOKIE="PHPSESSID=xxx; uid=xxx; pass=xxx"
   ```

### Integration with Radarr MVP

```rust
// Enhance search results with quality scores
impl HDBitsIndexer {
    pub async fn search_with_quality(&self, query: &str) -> Vec<Release> {
        let mut results = self.search(query).await?;
        
        // Add quality scores from analysis
        for release in &mut results {
            if let Some(score) = self.get_quality_score(release.id).await {
                release.quality_score = Some(score);
                release.quality_tier = Some(self.determine_tier(score));
            }
        }
        
        // Sort by quality
        results.sort_by(|a, b| 
            b.quality_score.partial_cmp(&a.quality_score).unwrap()
        );
        
        results
    }
}
```

## Monitoring & Maintenance

### Health Checks

```rust
// GET /api/analysis/health
{
    "status": "healthy",
    "last_collection": "2025-08-23T15:00:00Z",
    "torrents_processed_today": 1234,
    "collection_rate": 100,
    "error_rate": 0.02
}
```

### Maintenance Tasks

1. **Session Cookie Rotation** (Monthly)
   - Login to HDBits in browser
   - Extract new session cookie
   - Update services.env

2. **Database Cleanup** (Weekly)
   ```sql
   DELETE FROM quality_score_history 
   WHERE calculated_at < NOW() - INTERVAL '30 days';
   VACUUM ANALYZE torrent_analyses;
   ```

3. **Score Recalculation** (Monthly)
   ```bash
   cargo run --bin recalculate-scores --all
   ```

## Quality Guidelines Summary

### Must-Have Features for High Quality
- ✅ Trusted release group (P2P > Scene)
- ✅ Proper source (Blu-ray/Remux/WEB-DL)
- ✅ Modern codec (HEVC/H.264)
- ✅ Lossless audio track
- ✅ HDR for 4K content

### Red Flags to Avoid
- ❌ Unknown groups
- ❌ Outdated codecs (XviD, MPEG-2)
- ❌ Severely undersized files
- ❌ Missing lossless audio on HD
- ❌ 4K without HDR

### Elite Release Characteristics
- ⭐ EXCLUSIVE or INTERNAL tag
- ⭐ Elite P2P group
- ⭐ Remux or high-bitrate encode
- ⭐ Dolby Vision + HDR10
- ⭐ Atmos/DTS:X audio
- ⭐ Quality score > 85

## Troubleshooting

### Common Issues

**Authentication Failed (Status 5)**
- Verify passkey is correct
- Ensure 2FA is enabled
- Check username spelling

**Rate Limiting**
- Increase delay between requests
- Check for IP ban
- Use exponential backoff

**Checkpoint Recovery**
```bash
cat /tmp/radarr/analysis/checkpoint.json
python3 hdbits_resilient_analyzer.py  # Auto-resumes
```

## Future Enhancements

1. **Machine Learning Quality Prediction**
   - Train model on scored releases
   - Predict quality from release name
   - Identify anomalies and fakes

2. **Cross-Tracker Integration**
   - Import scores from other trackers
   - Unified quality database
   - Community voting system

3. **Automated Download Decisions**
   - Auto-grab releases with score > threshold
   - Replace lower quality with higher
   - Smart upgrade paths

---

*Last Updated: 2025-08-23*
*Data Collection Status: Active (29,225 torrents analyzed)*
*Next Update: Continuous - service runs 24/7*