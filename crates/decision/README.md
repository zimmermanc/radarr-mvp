# Radarr Decision Engine

Quality profiles and release selection logic for the Radarr movie automation system. This crate implements sophisticated decision-making algorithms to automatically select the best releases based on configurable quality profiles, custom formats, and user preferences.

## Features

- **Quality Profiles**: Flexible quality management with cutoff points and allowed qualities
- **Custom Formats**: Advanced release filtering based on custom criteria
- **Release Scoring**: Intelligent scoring system for automatic release selection
- **Decision Engine**: Rule-based decision making for download automation
- **Format Specifications**: Regex-based format detection and scoring
- **Quality Sources**: Support for various media sources (BluRay, WebDL, HDTV, etc.)
- **Upgrade Logic**: Automatic upgrades when better quality becomes available
- **Reject Reasons**: Detailed explanations for rejected releases

## Key Dependencies

- **radarr-core**: Core domain models and types
- **tokio**: Async runtime support
- **serde**: Configuration serialization/deserialization
- **uuid**: Unique identifiers for profiles and formats
- **regex**: Pattern matching for release analysis
- **tracing**: Structured logging for decision tracking

## Quality Management

### Quality Definitions

```rust
use radarr_decision::{Quality, Source};

// Supported quality levels (lowest to highest)
pub enum Quality {
    Unknown,
    Cam,
    Telesync,
    Telecine,
    WorkPrint,
    DVD,
    SDTV,
    WEBDL480p,
    HDTV720p,
    WEBDL720p,
    Bluray720p,
    HDTV1080p,
    WEBDL1080p,
    BlurayRaw,
    Bluray1080p,
    Remux1080p,
    HDTV2160p,
    WEBDL2160p,
    Bluray2160p,
    Remux2160p,
}

// Media sources
pub enum Source {
    Unknown,
    Cam,
    Telesync,
    Telecine,
    WorkPrint,
    DVD,
    TV,
    WebDL,
    WebRip,
    BluRay,
}
```

### Quality Profiles

```rust
use radarr_decision::{QualityProfile, QualityItem, Quality};
use uuid::Uuid;

// Create a quality profile
let profile = QualityProfile {
    id: Uuid::new_v4(),
    name: "Ultra HD".to_string(),
    cutoff: Quality::Remux2160p,
    items: vec![
        QualityItem {
            quality: Quality::Bluray1080p,
            allowed: true,
        },
        QualityItem {
            quality: Quality::WEBDL2160p,
            allowed: true,
        },
        QualityItem {
            quality: Quality::Remux2160p,
            allowed: true,
        },
    ],
    min_format_score: 0,
    cutoff_format_score: 10000,
    upgrade_allowed: true,
};

// Check if quality is allowed
let is_allowed = profile.is_quality_allowed(&Quality::Bluray1080p);
let needs_upgrade = profile.needs_upgrade(&current_quality, &new_quality);
```

## Custom Formats

### Format Specifications

```rust
use radarr_decision::{CustomFormat, FormatSpecification};

let format = CustomFormat {
    id: Uuid::new_v4(),
    name: "High Quality Groups".to_string(),
    specifications: vec![
        FormatSpecification::ReleaseName {
            pattern: r"-(SPARKS|FGT|DON|HONE)$".to_string(),
            negate: false,
        },
        FormatSpecification::IndexerFlags {
            required_flags: vec!["Internal".to_string()],
            forbidden_flags: vec!["Scene".to_string()],
        },
        FormatSpecification::Source {
            allowed_sources: vec![Source::BluRay],
        },
    ],
    score: 100,
};

// Test release against format
let release_data = ReleaseData {
    title: "Movie.2023.1080p.BluRay.x264-SPARKS".to_string(),
    indexer_flags: vec!["Internal".to_string()],
    source: Source::BluRay,
    // ... other fields
};

let matches = format.matches(&release_data)?;
if matches {
    println!("Release matches format, score: {}", format.score);
}
```

### Format Engine

```rust
use radarr_decision::{CustomFormatEngine, ReleaseData};

let engine = CustomFormatEngine::new();

// Add multiple custom formats
engine.add_format(high_quality_format);
engine.add_format(preferred_codec_format);
engine.add_format(unwanted_format);

// Calculate total format score for a release
let total_score = engine.calculate_score(&release_data)?;
println!("Total format score: {}", total_score);

// Get matching formats with details
let matches = engine.get_matching_formats(&release_data)?;
for format_match in matches {
    println!("Matched: {} (Score: {})", format_match.name, format_match.score);
}
```

## Decision Engine

### Release Evaluation

```rust
use radarr_decision::{DecisionEngine, Release, ReleaseScore};

let engine = DecisionEngine::new(quality_profile, custom_formats);

// Evaluate a release
let release = Release {
    title: "Movie.2023.2160p.BluRay.x265-GROUP".to_string(),
    quality: Quality::Bluray2160p,
    size_bytes: 15_000_000_000,
    indexer: "HDBits".to_string(),
    seeders: 50,
    age_hours: 2,
    // ... other fields
};

let decision = engine.evaluate_release(&release)?;

match decision {
    ReleaseScore::Accept { score, reasons } => {
        println!("Release accepted with score: {}", score);
        for reason in reasons {
            println!("  + {}", reason);
        }
    }
    ReleaseScore::Reject { reasons } => {
        println!("Release rejected:");
        for reason in reasons {
            println!("  - {}", reason);
        }
    }
    ReleaseScore::Pending { score, reasons } => {
        println!("Release pending (score: {})", score);
    }
}
```

### Batch Decision Making

```rust
// Evaluate multiple releases and rank them
let releases = vec![release1, release2, release3];
let decisions = engine.evaluate_batch(&releases)?;

// Get best release
let best_release = decisions
    .iter()
    .filter_map(|d| match d.score {
        ReleaseScore::Accept { score, .. } => Some((d.release, score)),
        _ => None,
    })
    .max_by_key(|(_, score)| *score);

if let Some((release, score)) = best_release {
    println!("Best release: {} (Score: {})", release.title, score);
}
```

## Advanced Features

### Upgrade Decision Logic

```rust
use radarr_decision::{UpgradeDecision, CurrentRelease};

let current = CurrentRelease {
    quality: Quality::Bluray1080p,
    format_score: 50,
    size_bytes: 8_000_000_000,
    // ... other fields
};

let candidate = Release {
    quality: Quality::Remux2160p,
    // ... other fields
};

let upgrade_decision = engine.should_upgrade(&current, &candidate)?;

match upgrade_decision {
    UpgradeDecision::Yes { improvement_score } => {
        println!("Upgrade recommended (improvement: {})", improvement_score);
    }
    UpgradeDecision::No { reason } => {
        println!("Upgrade not recommended: {}", reason);
    }
    UpgradeDecision::Maybe { conditions } => {
        println!("Conditional upgrade - conditions: {:?}", conditions);
    }
}
```

### Size-Based Filtering

```rust
use radarr_decision::SizeFilter;

let size_filter = SizeFilter {
    min_size_mb: Some(100),      // Minimum 100MB
    max_size_mb: Some(25000),    // Maximum 25GB
    preferred_size_mb: Some(8000), // Prefer around 8GB
    size_weight: 0.1,            // 10% of total score
};

// Apply size filtering to releases
let size_score = size_filter.calculate_score(release.size_bytes);
```

### Seeder and Age Preferences

```rust
use radarr_decision::PreferenceFilters;

let preferences = PreferenceFilters {
    min_seeders: Some(5),
    preferred_seeders: Some(50),
    max_age_hours: Some(24 * 7), // 1 week
    prefer_newer: true,
    seeder_weight: 0.2,          // 20% of score
    age_weight: 0.1,             // 10% of score
};

let preference_score = preferences.calculate_score(&release);
```

## Configuration Examples

### Ultra HD Profile

```json
{
  "name": "Ultra HD",
  "cutoff": "Remux2160p",
  "items": [
    {"quality": "WEBDL2160p", "allowed": true},
    {"quality": "Bluray2160p", "allowed": true},
    {"quality": "Remux2160p", "allowed": true}
  ],
  "minFormatScore": 0,
  "cutoffFormatScore": 10000,
  "upgradeAllowed": true
}
```

### High Quality Format

```json
{
  "name": "High Quality Groups",
  "score": 100,
  "specifications": [
    {
      "type": "ReleaseName",
      "pattern": "-(SPARKS|FGT|DON|HONE)$",
      "negate": false
    },
    {
      "type": "Source",
      "allowedSources": ["BluRay"]
    }
  ]
}
```

## Testing

### Unit Tests

```bash
# Run decision engine tests
cargo test -p radarr-decision

# Test specific modules
cargo test -p radarr-decision quality::tests
cargo test -p radarr-decision engine::tests
```

### Example Test Cases

```rust
use radarr_decision::{Quality, QualityProfile, DecisionEngine};

#[test]
fn test_quality_profile_cutoff() {
    let profile = create_test_profile();
    
    // Should allow qualities below cutoff
    assert!(profile.is_quality_allowed(&Quality::Bluray1080p));
    
    // Should not allow qualities above cutoff
    assert!(!profile.is_quality_allowed(&Quality::Remux2160p));
}

#[tokio::test]
async fn test_release_evaluation() {
    let engine = DecisionEngine::new(test_profile(), test_formats());
    
    let release = create_test_release();
    let decision = engine.evaluate_release(&release).unwrap();
    
    match decision {
        ReleaseScore::Accept { score, .. } => {
            assert!(score > 0);
        }
        _ => panic!("Expected acceptance"),
    }
}
```

## Integration

### With Core Domain

```rust
use radarr_core::{Movie, MovieRepository};
use radarr_decision::DecisionEngine;

async fn process_releases(
    movie: &Movie,
    releases: Vec<Release>,
    engine: &DecisionEngine,
) -> Result<Option<Release>> {
    let decisions = engine.evaluate_batch(&releases)?;
    
    // Find best release for this movie
    let best_release = decisions
        .into_iter()
        .filter_map(|d| match d.score {
            ReleaseScore::Accept { score, .. } => Some((d.release, score)),
            _ => None,
        })
        .max_by_key(|(_, score)| *score)
        .map(|(release, _)| release);
    
    Ok(best_release)
}
```

### With Indexers

```rust
use radarr_indexers::IndexerResult;
use radarr_decision::Release;

// Convert indexer results to decision engine releases
fn convert_to_releases(indexer_results: Vec<IndexerResult>) -> Vec<Release> {
    indexer_results
        .into_iter()
        .map(|result| Release {
            title: result.title,
            quality: result.quality,
            size_bytes: result.size_bytes,
            indexer: result.indexer,
            seeders: result.seeders.unwrap_or(0),
            // ... convert other fields
        })
        .collect()
}
```

## Performance Considerations

- **Efficient Regex**: Compiled regex patterns cached for reuse
- **Parallel Evaluation**: Batch processing with async/await
- **Memory Optimization**: Minimal allocations during evaluation
- **Scoring Cache**: Cache format scores for repeated evaluations
- **Profile Validation**: Compile-time validation of profile configurations