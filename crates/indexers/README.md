# Radarr Indexers

This crate provides integration with various torrent and NZB indexers for the Radarr media management system.

## Supported Indexers

### HDBits
- **Type**: Private BitTorrent tracker
- **Authentication**: Username + Passkey 
- **Rate Limiting**: 150 requests per hour (configurable)
- **Features**:
  - Movie search by title, year, or IMDB ID
  - Quality parsing from release names (resolution, source, codec, audio, HDR)
  - Scene group extraction
  - Rate limiting with automatic retry
  - Passkey-based authentication (no session cookies)

### Prowlarr (Existing)
- **Type**: Indexer proxy
- **Authentication**: API key
- **Features**: Multi-indexer search aggregation

## Usage

### HDBits Direct Integration

```rust
use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};

// Create client with default credentials
let config = HDBitsConfig::default();
let client = HDBitsClient::new(config)?;

// Search for movies
let request = MovieSearchRequest::new()
    .with_title("Dune")
    .with_year(2021)
    .with_limit(10)
    .with_min_seeders(5);

let releases = client.search_movies(&request).await?;

for release in releases {
    println!("📦 {}", release.title);
    println!("   Size: {}", release.human_readable_size().unwrap_or("Unknown".to_string()));
    println!("   Quality: {}", serde_json::to_string_pretty(&release.quality)?);
}
```

### Generic IndexerClient Interface

```rust
use radarr_indexers::{HDBitsClient, IndexerClient, SearchRequest};

let client = HDBitsClient::from_env()?;

let request = SearchRequest {
    query: Some("Blade Runner 2049".to_string()),
    categories: vec![2000], // Movies
    limit: Some(5),
    min_seeders: Some(10),
    ..Default::default()
};

let response = client.search(&request).await?;
println!("Found {} results", response.total);
```

## Configuration

### Environment Variables

```bash
export HDBITS_USERNAME="your_username"
export HDBITS_PASSKEY="your_128_char_passkey"
export HDBITS_API_URL="https://hdbits.org/api/torrents"
export HDBITS_RATE_LIMIT="150"  # requests per hour
export HDBITS_TIMEOUT="30"      # seconds
```

### Programmatic Configuration

```rust
let config = HDBitsConfig {
    username: "your_username".to_string(),
    passkey: "your_128_char_passkey".to_string(),
    api_url: "https://hdbits.org/api/torrents".to_string(),
    rate_limit_per_hour: 150,
    timeout_seconds: 30,
};

let client = HDBitsClient::new(config)?;
```

## Quality Parsing

The HDBits indexer automatically parses quality information from release names:

```rust
// Input: "Dune.2021.2160p.UHD.BluRay.x265.HDR10.Atmos-GROUP"
// Parsed quality:
{
  "resolution": "2160p",
  "source": "BluRay", 
  "codec": "x265",
  "hdr": "HDR10",
  "audio": "Atmos",
  "score": 195,
  "indexer": "HDBits",
  "scene_group": "GROUP"
}
```

## Rate Limiting

HDBits enforces a 150 requests per hour limit. The client includes automatic rate limiting:

- Tracks request timestamps over rolling 1-hour window
- Automatically waits when limit exceeded
- Transparent to calling code - just returns when safe to proceed

## Error Handling

The indexer provides detailed error handling:

```rust
match client.search_movies(&request).await {
    Ok(releases) => println!("Found {} releases", releases.len()),
    Err(RadarrError::ExternalServiceError { service, error }) => {
        println!("HDBits error: {}", error);
    }
    Err(RadarrError::NotFound { resource }) => {
        println!("No results found");
    }
    Err(e) => println!("Other error: {}", e),
}
```

## Testing

Run unit tests:
```bash
cargo test -p radarr-indexers hdbits
```

Run example (requires valid HDBits credentials):
```bash
cargo run --example hdbits_example
```

## Integration

The HDBits indexer integrates seamlessly with the existing Radarr architecture:

1. **Core Models**: Uses `Release` and `ReleaseProtocol` from `radarr-core`
2. **IndexerClient Trait**: Implements the common `IndexerClient` interface
3. **API Layer**: Can be wired into existing API endpoints
4. **Repository Pattern**: Releases can be stored via existing repository abstractions

The implementation follows clean architecture principles with proper separation of concerns and dependency injection support.