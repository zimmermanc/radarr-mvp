# API Documentation

Complete REST API reference for Radarr MVP with examples, authentication, and integration guides.

## Table of Contents

1. [Authentication](#authentication)
2. [Base URLs and Versioning](#base-urls-and-versioning)
3. [Response Formats](#response-formats)
4. [Error Handling](#error-handling)
5. [Rate Limiting](#rate-limiting)
6. [API Endpoints](#api-endpoints)
7. [WebSocket API](#websocket-api)
8. [SDK and Examples](#sdk-and-examples)

## Authentication

### API Key Authentication

All API requests require authentication via API key in the request header:

```http
X-Api-Key: your-api-key-here
```

**Examples:**

```bash
# Using curl
curl -H "X-Api-Key: abc123" http://localhost:7878/api/v3/movie

# Using JavaScript fetch
fetch('http://localhost:7878/api/v3/movie', {
  headers: {
    'X-Api-Key': 'abc123',
    'Content-Type': 'application/json'
  }
})

# Using Python requests
import requests
headers = {'X-Api-Key': 'abc123'}
response = requests.get('http://localhost:7878/api/v3/movie', headers=headers)
```

### API Key Management

**Generate New API Key:**
```bash
# Using the CLI tool
cargo run --bin generate-api-key

# Or manually
openssl rand -hex 32
```

**Configure API Key:**
```bash
# In .env file
API_KEY=your-secure-api-key-here

# Or as environment variable
export API_KEY=your-secure-api-key-here
```

## Base URLs and Versioning

**Base URL:** `http://localhost:7878`

**API Versions:**
- `/api/v1/` - Legacy compatibility endpoints
- `/api/v3/` - Current API version (recommended)

**Health Endpoints:**
- `/health` - Basic health check
- `/health/detailed` - Detailed component health

## Response Formats

### Success Responses

**Single Resource:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "title": "The Matrix",
  "year": 1999,
  "tmdb_id": 603,
  "monitored": true,
  "status": "downloaded",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T15:45:00Z"
}
```

**Collection Response:**
```json
{
  "data": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "title": "The Matrix",
      "year": 1999
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 50,
    "total": 156,
    "pages": 4
  }
}
```

### HTTP Status Codes

| Code | Meaning | Usage |
|------|---------|-------|
| `200` | OK | Successful GET, PUT requests |
| `201` | Created | Successful POST requests |
| `204` | No Content | Successful DELETE requests |
| `400` | Bad Request | Invalid request parameters |
| `401` | Unauthorized | Missing or invalid API key |
| `404` | Not Found | Resource does not exist |
| `409` | Conflict | Resource already exists |
| `422` | Unprocessable Entity | Validation errors |
| `429` | Too Many Requests | Rate limit exceeded |
| `500` | Internal Server Error | Server error |
| `503` | Service Unavailable | External service unavailable |

## Error Handling

### Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed for the request",
    "details": [
      {
        "field": "title",
        "message": "Title is required"
      },
      {
        "field": "tmdb_id",
        "message": "TMDB ID must be a positive integer"
      }
    ]
  },
  "request_id": "req_123456789"
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `VALIDATION_ERROR` | Request validation failed |
| `AUTHENTICATION_ERROR` | Invalid or missing API key |
| `NOT_FOUND` | Resource not found |
| `DUPLICATE_RESOURCE` | Resource already exists |
| `EXTERNAL_SERVICE_ERROR` | External service (Prowlarr, qBittorrent) error |
| `DATABASE_ERROR` | Database operation failed |
| `RATE_LIMIT_EXCEEDED` | Too many requests |

## Rate Limiting

**Default Limits:**
- 100 requests per minute per IP
- 20 burst requests allowed
- Rate limit headers included in responses

**Rate Limit Headers:**
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1642680000
Retry-After: 60
```

## API Endpoints

### Health and System

#### GET /health

Basic health check endpoint.

**Response:**
```json
{
  "service": "radarr-api",
  "status": "healthy",
  "version": "1.0.0",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### GET /health/detailed

Detailed health check with component status.

**Response:**
```json
{
  "service": "radarr-api",
  "status": "healthy",
  "version": "1.0.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "components": {
    "database": {
      "status": "healthy",
      "response_time": "2ms",
      "connections": {
        "active": 5,
        "max": 20
      }
    },
    "prowlarr": {
      "status": "healthy",
      "response_time": "150ms",
      "url": "http://localhost:9696"
    },
    "qbittorrent": {
      "status": "healthy",
      "response_time": "45ms",
      "url": "http://localhost:8080"
    }
  }
}
```

#### GET /api/v1/system/status

Legacy system status endpoint for compatibility.

**Response:**
```json
{
  "service": "radarr-mvp",
  "version": "1.0.0",
  "uptime": "2d 5h 30m",
  "config": {
    "prowlarr_configured": true,
    "qbittorrent_configured": true,
    "import_enabled": true
  }
}
```

### Movie Management

#### GET /api/v3/movie

Retrieve a list of movies with pagination.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | Integer | `1` | Page number |
| `limit` | Integer | `50` | Items per page (max 100) |
| `sort` | String | `title` | Sort field (title, year, added) |
| `order` | String | `asc` | Sort order (asc, desc) |
| `monitored` | Boolean | None | Filter by monitored status |
| `status` | String | None | Filter by status (wanted, downloaded, missing) |
| `search` | String | None | Search by title |

**Example Request:**
```bash
curl -H "X-Api-Key: abc123" \
  "http://localhost:7878/api/v3/movie?page=1&limit=10&monitored=true&sort=year&order=desc"
```

**Response:**
```json
{
  "data": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "title": "The Matrix",
      "year": 1999,
      "tmdb_id": 603,
      "imdb_id": "tt0133093",
      "monitored": true,
      "status": "downloaded",
      "quality_profile": {
        "id": "profile_001",
        "name": "HD-1080p"
      },
      "poster_url": "https://image.tmdb.org/t/p/w500/poster.jpg",
      "overview": "A computer programmer discovers...",
      "runtime": 136,
      "genres": ["Action", "Science Fiction"],
      "ratings": {
        "tmdb": 8.2,
        "imdb": 8.7
      },
      "files": [
        {
          "id": "file_001",
          "path": "/movies/The Matrix (1999)/The Matrix (1999) - Bluray-1080p.mkv",
          "size": 15728640000,
          "quality": "Bluray-1080p",
          "media_info": {
            "codec": "x264",
            "resolution": "1920x1080",
            "audio": "DTS-HD MA"
          }
        }
      ],
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T15:45:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total": 156,
    "pages": 16
  }
}
```

#### GET /api/v3/movie/{id}

Retrieve a specific movie by UUID.

**Path Parameters:**
- `id` (UUID, required) - Movie UUID

**Example Request:**
```bash
curl -H "X-Api-Key: abc123" \
  http://localhost:7878/api/v3/movie/123e4567-e89b-12d3-a456-426614174000
```

**Response:**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "title": "The Matrix",
  "year": 1999,
  "tmdb_id": 603,
  "imdb_id": "tt0133093",
  "monitored": true,
  "status": "downloaded",
  "quality_profile": {
    "id": "profile_001",
    "name": "HD-1080p"
  },
  "download_history": [
    {
      "id": "download_001",
      "release_title": "The.Matrix.1999.1080p.BluRay.x264-GROUP",
      "indexer": "Example Tracker",
      "download_client": "qBittorrent",
      "downloaded_at": "2024-01-15T12:00:00Z",
      "status": "completed"
    }
  ]
}
```

#### POST /api/v3/movie

Add a new movie to the library.

**Request Body:**
```json
{
  "tmdb_id": 603,
  "title": "The Matrix",
  "year": 1999,
  "monitored": true,
  "quality_profile_id": "profile_001",
  "search_on_add": true
}
```

**Required Fields:**
- `tmdb_id` (Integer) - TMDB movie ID
- `title` (String) - Movie title
- `monitored` (Boolean) - Whether to monitor for releases

**Optional Fields:**
- `year` (Integer) - Release year
- `quality_profile_id` (String) - Quality profile UUID (uses default if not specified)
- `search_on_add` (Boolean, default: true) - Search for releases immediately after adding

**Example Request:**
```bash
curl -X POST -H "X-Api-Key: abc123" -H "Content-Type: application/json" \
  -d '{
    "tmdb_id": 603,
    "title": "The Matrix",
    "year": 1999,
    "monitored": true,
    "search_on_add": true
  }' \
  http://localhost:7878/api/v3/movie
```

**Response (201 Created):**
```json
{
  "id": "456e7890-e89b-12d3-a456-426614174111",
  "tmdb_id": 603,
  "title": "The Matrix",
  "year": 1999,
  "monitored": true,
  "status": "wanted",
  "quality_profile": {
    "id": "profile_001",
    "name": "HD-1080p"
  },
  "created_at": "2024-01-15T16:30:00Z",
  "updated_at": "2024-01-15T16:30:00Z"
}
```

#### PUT /api/v3/movie/{id}

Update an existing movie.

**Request Body:**
```json
{
  "monitored": false,
  "quality_profile_id": "profile_002"
}
```

**Example Request:**
```bash
curl -X PUT -H "X-Api-Key: abc123" -H "Content-Type: application/json" \
  -d '{"monitored": false}' \
  http://localhost:7878/api/v3/movie/123e4567-e89b-12d3-a456-426614174000
```

#### DELETE /api/v3/movie/{id}

Remove a movie from the library.

**Query Parameters:**
- `delete_files` (Boolean, default: false) - Also delete associated files

**Example Request:**
```bash
curl -X DELETE -H "X-Api-Key: abc123" \
  "http://localhost:7878/api/v3/movie/123e4567-e89b-12d3-a456-426614174000?delete_files=true"
```

**Response:** `204 No Content`

### Search and Discovery

#### GET /api/v3/movie/lookup

Search for movies on TMDB.

**Query Parameters:**
- `term` (String, required) - Search term
- `year` (Integer, optional) - Filter by release year

**Example Request:**
```bash
curl -H "X-Api-Key: abc123" \
  "http://localhost:7878/api/v3/movie/lookup?term=matrix&year=1999"
```

**Response:**
```json
{
  "data": [
    {
      "tmdb_id": 603,
      "title": "The Matrix",
      "year": 1999,
      "overview": "A computer programmer discovers...",
      "poster_url": "https://image.tmdb.org/t/p/w500/poster.jpg",
      "backdrop_url": "https://image.tmdb.org/t/p/w1280/backdrop.jpg",
      "genres": ["Action", "Science Fiction"],
      "runtime": 136,
      "ratings": {
        "tmdb": 8.2
      },
      "in_library": false
    }
  ]
}
```

#### POST /api/v3/indexer/search

Search for movie releases across configured indexers.

**Request Body:**
```json
{
  "movie_id": "123e4567-e89b-12d3-a456-426614174000",
  "categories": ["Movies/HD", "Movies/UHD"],
  "min_size": 1000,
  "max_size": 50000
}
```

**Example Request:**
```bash
curl -X POST -H "X-Api-Key: abc123" -H "Content-Type: application/json" \
  -d '{
    "movie_id": "123e4567-e89b-12d3-a456-426614174000",
    "categories": ["Movies/HD"]
  }' \
  http://localhost:7878/api/v3/indexer/search
```

**Response:**
```json
{
  "data": [
    {
      "id": "release_001",
      "title": "The.Matrix.1999.1080p.BluRay.x264-GROUP",
      "indexer": "Example Tracker",
      "category": "Movies/HD",
      "size": 15728640000,
      "age": 1825,
      "seeders": 150,
      "leechers": 5,
      "download_url": "magnet:?xt=urn:btih:...",
      "info_url": "https://tracker.example.com/details/12345",
      "quality": {
        "resolution": "1080p",
        "source": "BluRay",
        "codec": "x264"
      },
      "languages": ["en"],
      "score": 85
    }
  ]
}
```

### Download Management

#### POST /api/v3/download

Start a download.

**Request Body:**
```json
{
  "release_id": "release_001",
  "movie_id": "123e4567-e89b-12d3-a456-426614174000",
  "download_url": "magnet:?xt=urn:btih:...",
  "category": "movies"
}
```

**Example Request:**
```bash
curl -X POST -H "X-Api-Key: abc123" -H "Content-Type: application/json" \
  -d '{
    "release_id": "release_001",
    "movie_id": "123e4567-e89b-12d3-a456-426614174000",
    "download_url": "magnet:?xt=urn:btih:..."
  }' \
  http://localhost:7878/api/v3/download
```

**Response (201 Created):**
```json
{
  "id": "download_001",
  "status": "queued",
  "progress": 0,
  "download_client": "qBittorrent",
  "created_at": "2024-01-15T16:45:00Z"
}
```

#### GET /api/v3/download

List active downloads.

**Response:**
```json
{
  "data": [
    {
      "id": "download_001",
      "movie_id": "123e4567-e89b-12d3-a456-426614174000",
      "movie_title": "The Matrix",
      "release_title": "The.Matrix.1999.1080p.BluRay.x264-GROUP",
      "status": "downloading",
      "progress": 65.5,
      "download_speed": 5242880,
      "eta": 1800,
      "download_client": "qBittorrent",
      "created_at": "2024-01-15T16:45:00Z",
      "updated_at": "2024-01-15T17:30:00Z"
    }
  ]
}
```

#### DELETE /api/v3/download/{id}

Cancel and remove a download.

**Query Parameters:**
- `remove_files` (Boolean, default: false) - Also remove downloaded files

**Example Request:**
```bash
curl -X DELETE -H "X-Api-Key: abc123" \
  "http://localhost:7878/api/v3/download/download_001?remove_files=true"
```

### Calendar and Feeds

#### GET /api/v3/calendar

Get upcoming movie releases.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `start` | Date | Today | Start date (YYYY-MM-DD) |
| `end` | Date | +30 days | End date (YYYY-MM-DD) |
| `monitored` | Boolean | None | Filter by monitored status |

**Example Request:**
```bash
curl -H "X-Api-Key: abc123" \
  "http://localhost:7878/api/v3/calendar?start=2024-01-15&end=2024-02-15&monitored=true"
```

**Response:**
```json
{
  "data": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "title": "The Matrix",
      "year": 1999,
      "status": "wanted",
      "monitored": true,
      "release_date": "2024-01-20",
      "poster_url": "https://image.tmdb.org/t/p/w500/poster.jpg"
    }
  ]
}
```

#### GET /feed/v3/calendar/radarr.ics

iCal feed for calendar integration.

**Query Parameters:**
- `apikey` (String, required) - API key for authentication
- `monitored` (Boolean, default: true) - Filter by monitored status
- `days` (Integer, default: 30) - Number of days to include

**Example Request:**
```bash
curl "http://localhost:7878/feed/v3/calendar/radarr.ics?apikey=abc123&days=60"
```

**Response:** Standard iCal format
```
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Radarr MVP//Calendar//EN
BEGIN:VEVENT
UID:movie-123e4567-e89b-12d3-a456-426614174000
DTSTAMP:20240115T103000Z
DTSTART:20240120T000000Z
SUMMARY:The Matrix (1999)
DESCRIPTION:A computer programmer discovers...
END:VEVENT
END:VCALENDAR
```

### Quality Profiles

#### GET /api/v3/qualityprofile

List quality profiles.

**Response:**
```json
{
  "data": [
    {
      "id": "profile_001",
      "name": "HD-1080p",
      "cutoff": "Bluray-1080p",
      "items": [
        {
          "quality": "HDTV-720p",
          "allowed": true,
          "preferred": false
        },
        {
          "quality": "Bluray-1080p",
          "allowed": true,
          "preferred": true
        }
      ],
      "min_format_score": 0,
      "upgrade_allowed": true,
      "language": "en"
    }
  ]
}
```

#### POST /api/v3/qualityprofile

Create a new quality profile.

**Request Body:**
```json
{
  "name": "4K UHD",
  "cutoff": "Bluray-2160p",
  "items": [
    {
      "quality": "WEBDL-2160p",
      "allowed": true,
      "preferred": false
    },
    {
      "quality": "Bluray-2160p",
      "allowed": true,
      "preferred": true
    }
  ],
  "upgrade_allowed": true
}
```

### Commands and Tasks

#### POST /api/v3/command

Execute system commands.

**Available Commands:**
- `search` - Search for specific movie
- `import` - Trigger import scan
- `refresh` - Refresh movie metadata
- `backup` - Create system backup

**Request Body:**
```json
{
  "name": "search",
  "movie_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

**Example Request:**
```bash
curl -X POST -H "X-Api-Key: abc123" -H "Content-Type: application/json" \
  -d '{
    "name": "import"
  }' \
  http://localhost:7878/api/v3/command
```

**Response:**
```json
{
  "id": "cmd_001",
  "name": "import",
  "status": "queued",
  "created_at": "2024-01-15T17:00:00Z"
}
```

#### GET /api/v3/command/{id}

Get command status.

**Response:**
```json
{
  "id": "cmd_001",
  "name": "import",
  "status": "completed",
  "progress": 100,
  "result": {
    "imported_files": 5,
    "skipped_files": 1,
    "errors": 0
  },
  "created_at": "2024-01-15T17:00:00Z",
  "completed_at": "2024-01-15T17:05:00Z"
}
```

## WebSocket API

### Connection

**Endpoint:** `ws://localhost:7878/ws`

**Authentication:** Include API key in connection headers or query parameter

```javascript
// JavaScript WebSocket connection
const ws = new WebSocket('ws://localhost:7878/ws?apikey=abc123');

// Or with headers
const ws = new WebSocket('ws://localhost:7878/ws', [], {
  headers: {
    'X-Api-Key': 'abc123'
  }
});
```

### Message Types

#### Real-time Updates

**Download Progress:**
```json
{
  "type": "download_progress",
  "data": {
    "id": "download_001",
    "progress": 75.5,
    "download_speed": 5242880,
    "eta": 900
  }
}
```

**Import Events:**
```json
{
  "type": "import_complete",
  "data": {
    "movie_id": "123e4567-e89b-12d3-a456-426614174000",
    "file_path": "/movies/The Matrix (1999)/The Matrix (1999).mkv",
    "quality": "Bluray-1080p"
  }
}
```

**System Events:**
```json
{
  "type": "system_status",
  "data": {
    "component": "prowlarr",
    "status": "offline",
    "message": "Connection timeout"
  }
}
```

## SDK and Examples

### JavaScript/TypeScript SDK

```typescript
class RadarrAPI {
  private baseUrl: string;
  private apiKey: string;

  constructor(baseUrl: string, apiKey: string) {
    this.baseUrl = baseUrl;
    this.apiKey = apiKey;
  }

  private async request(endpoint: string, options: RequestInit = {}): Promise<any> {
    const url = `${this.baseUrl}${endpoint}`;
    const response = await fetch(url, {
      ...options,
      headers: {
        'X-Api-Key': this.apiKey,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async getMovies(params: {
    page?: number;
    limit?: number;
    monitored?: boolean;
  } = {}): Promise<Movie[]> {
    const query = new URLSearchParams(params as any).toString();
    const response = await this.request(`/api/v3/movie?${query}`);
    return response.data;
  }

  async addMovie(movie: {
    tmdb_id: number;
    title: string;
    monitored: boolean;
  }): Promise<Movie> {
    return this.request('/api/v3/movie', {
      method: 'POST',
      body: JSON.stringify(movie),
    });
  }

  async searchMovie(term: string): Promise<SearchResult[]> {
    const response = await this.request(`/api/v3/movie/lookup?term=${encodeURIComponent(term)}`);
    return response.data;
  }
}

// Usage
const radarr = new RadarrAPI('http://localhost:7878', 'your-api-key');

// Get all movies
const movies = await radarr.getMovies({ monitored: true });

// Add a movie
const newMovie = await radarr.addMovie({
  tmdb_id: 603,
  title: 'The Matrix',
  monitored: true
});

// Search for movies
const searchResults = await radarr.searchMovie('matrix');
```

### Python SDK

```python
import requests
from typing import List, Dict, Optional

class RadarrAPI:
    def __init__(self, base_url: str, api_key: str):
        self.base_url = base_url.rstrip('/')
        self.api_key = api_key
        self.session = requests.Session()
        self.session.headers.update({
            'X-Api-Key': api_key,
            'Content-Type': 'application/json'
        })

    def _request(self, method: str, endpoint: str, **kwargs) -> Dict:
        url = f"{self.base_url}{endpoint}"
        response = self.session.request(method, url, **kwargs)
        response.raise_for_status()
        return response.json()

    def get_movies(self, page: int = 1, limit: int = 50, 
                   monitored: Optional[bool] = None) -> List[Dict]:
        params = {'page': page, 'limit': limit}
        if monitored is not None:
            params['monitored'] = monitored
        
        response = self._request('GET', '/api/v3/movie', params=params)
        return response['data']

    def add_movie(self, tmdb_id: int, title: str, monitored: bool = True) -> Dict:
        data = {
            'tmdb_id': tmdb_id,
            'title': title,
            'monitored': monitored
        }
        return self._request('POST', '/api/v3/movie', json=data)

    def search_movie(self, term: str) -> List[Dict]:
        params = {'term': term}
        response = self._request('GET', '/api/v3/movie/lookup', params=params)
        return response['data']

    def get_health(self) -> Dict:
        return self._request('GET', '/health/detailed')

# Usage
radarr = RadarrAPI('http://localhost:7878', 'your-api-key')

# Get all monitored movies
movies = radarr.get_movies(monitored=True)

# Add a movie
new_movie = radarr.add_movie(tmdb_id=603, title='The Matrix')

# Search for movies
search_results = radarr.search_movie('matrix')

# Check system health
health = radarr.get_health()
print(f"System status: {health['status']}")
```

### Shell/Bash Examples

```bash
#!/bin/bash

# Configuration
RADARR_URL="http://localhost:7878"
API_KEY="your-api-key-here"

# Helper function for API calls
api_call() {
    local method="$1"
    local endpoint="$2"
    local data="$3"
    
    curl -s -X "$method" \
         -H "X-Api-Key: $API_KEY" \
         -H "Content-Type: application/json" \
         ${data:+-d "$data"} \
         "$RADARR_URL$endpoint"
}

# Get system health
health=$(api_call GET "/health/detailed")
echo "System Status: $(echo "$health" | jq -r '.status')"

# List movies
movies=$(api_call GET "/api/v3/movie?limit=10")
echo "Movies in library: $(echo "$movies" | jq '.pagination.total')"

# Search for a movie
search_results=$(api_call GET "/api/v3/movie/lookup?term=matrix")
echo "Search results: $(echo "$search_results" | jq '.data | length')"

# Add a movie
add_movie_data='{
  "tmdb_id": 603,
  "title": "The Matrix",
  "monitored": true
}'

new_movie=$(api_call POST "/api/v3/movie" "$add_movie_data")
echo "Added movie: $(echo "$new_movie" | jq -r '.title')"

# Check download status
downloads=$(api_call GET "/api/v3/download")
echo "Active downloads: $(echo "$downloads" | jq '.data | length')"
```

### Integration Examples

#### Webhook Handler (Node.js)

```javascript
const express = require('express');
const app = express();

app.use(express.json());

// Handle Radarr webhooks
app.post('/webhook/radarr', (req, res) => {
  const { eventType, movie, movieFile } = req.body;
  
  switch (eventType) {
    case 'Download':
      console.log(`Downloaded: ${movie.title} (${movieFile.quality})`);
      // Send notification, update database, etc.
      break;
      
    case 'MovieAdded':
      console.log(`New movie added: ${movie.title}`);
      break;
      
    case 'ImportComplete':
      console.log(`Import complete: ${movieFile.relativePath}`);
      break;
  }
  
  res.status(200).send('OK');
});

app.listen(3000, () => {
  console.log('Webhook handler listening on port 3000');
});
```

#### Monitoring Script (Python)

```python
import time
import logging
from radarr_api import RadarrAPI  # Custom SDK from above

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class RadarrMonitor:
    def __init__(self, api_url: str, api_key: str):
        self.api = RadarrAPI(api_url, api_key)
        self.last_check = {}
    
    def check_health(self):
        try:
            health = self.api.get_health()
            if health['status'] != 'healthy':
                logger.warning(f"System health: {health['status']}")
                for component, status in health['components'].items():
                    if status['status'] != 'healthy':
                        logger.error(f"Component {component} is {status['status']}")
            return health['status'] == 'healthy'
        except Exception as e:
            logger.error(f"Health check failed: {e}")
            return False
    
    def check_downloads(self):
        try:
            downloads = self.api._request('GET', '/api/v3/download')['data']
            for download in downloads:
                download_id = download['id']
                progress = download['progress']
                
                if download_id not in self.last_check:
                    logger.info(f"New download: {download['movie_title']}")
                elif progress > self.last_check[download_id]:
                    logger.info(f"Download progress: {download['movie_title']} - {progress:.1f}%")
                
                self.last_check[download_id] = progress
                
                if download['status'] == 'completed':
                    logger.info(f"Download completed: {download['movie_title']}")
                    del self.last_check[download_id]
        except Exception as e:
            logger.error(f"Download check failed: {e}")
    
    def run(self, interval: int = 30):
        logger.info("Starting Radarr monitor")
        while True:
            self.check_health()
            self.check_downloads()
            time.sleep(interval)

if __name__ == '__main__':
    monitor = RadarrMonitor('http://localhost:7878', 'your-api-key')
    monitor.run()
```

For more examples and integration patterns, see the [Contributing Guide](CONTRIBUTING.md) and [Installation Guide](INSTALL.md).