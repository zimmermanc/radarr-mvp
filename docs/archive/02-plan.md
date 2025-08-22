# Radarr MVP Advanced Features: ML/AI Discovery & Graph-Based Recommendations

**Generated**: 2025-08-20  
**Document Type**: Future Roadmap & Implementation Plan  
**Focus**: Advanced ML/AI Features, ArangoDB Graph Integration, Plex Placeholder System  
**Target Timeline**: 16-20 weeks post-MVP completion  

## Executive Summary

This document outlines the implementation strategy for advanced ML/AI-powered content discovery features that were initially conceived for the Radarr MVP but deferred to focus on core functionality. The plan centers around leveraging **ArangoDB's native graph capabilities** to build sophisticated recommendation systems that understand complex relationships between movies, actors, directors, genres, and user preferences.

### Core Vision
Transform Radarr from a media management tool into an **intelligent content discovery platform** that:
- Learns user taste patterns through viewing history and interactions
- Builds a comprehensive knowledge graph of media relationships
- Generates personalized recommendations using hybrid ML algorithms
- Seamlessly integrates suggestions into Plex via placeholder mechanism
- Enables one-click additions through watchlistarr-rust integration

---

## 🎯 Strategic Goals

### Primary Objectives
1. **Intelligent Discovery**: ML-powered recommendations that evolve with user taste
2. **Graph Intelligence**: Deep relationship understanding via ArangoDB
3. **Seamless UX**: Plex placeholders for frictionless content discovery
4. **Learning System**: Continuous improvement through feedback loops
5. **Performance**: Sub-100ms recommendation generation

### Success Metrics
- **Recommendation Accuracy**: >85% user satisfaction rate
- **Discovery Rate**: 3-5 new titles accepted per user/month
- **Graph Traversal**: <50ms for 3-depth relationship queries
- **Learning Efficiency**: Meaningful preferences after 10 interactions
- **System Scale**: Support 100K+ movies, 1M+ relationships

---

## 🏗️ Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                        User Interface                        │
│                    (Plex + Radarr Web UI)                   │
└─────────────────┬───────────────────────────┬───────────────┘
                  │                           │
┌─────────────────▼─────────────┐ ┌──────────▼──────────────┐
│   Plex Placeholder Manager    │ │   Recommendation API     │
│  (Virtual entries for recs)   │ │  (REST + GraphQL + WS)   │
└─────────────────┬─────────────┘ └──────────┬──────────────┘
                  │                           │
┌─────────────────▼───────────────────────────▼───────────────┐
│              ML/AI Recommendation Engine                     │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐       │
│  │Collaborative │ │Content-Based │ │Reinforcement │       │
│  │  Filtering   │ │   Filtering  │ │   Learning   │       │
│  └──────────────┘ └──────────────┘ └──────────────┘       │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    ArangoDB Graph Layer                      │
│  ┌────────────────────────────────────────────────────┐    │
│  │        Multi-Model Graph Database                   │    │
│  │  • Vertices: Movies, Actors, Directors, Genres      │    │
│  │  • Edges: acted_in, directed, similar_to, watched   │    │
│  │  • Documents: User profiles, preferences, history   │    │
│  │  • Vectors: Embeddings for semantic search (2024)   │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                   Data Integration Layer                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │
│  │   TMDB   │ │   Plex   │ │  Trakt   │ │  HDBits  │     │
│  │   API    │ │  Watch   │ │  History │ │  Scene   │     │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘     │
└──────────────────────────────────────────────────────────────┘
```

---

## 🔀 ArangoDB Graph Model Design

### Core Graph Structure

#### Vertices (Nodes)
```javascript
// Movie Vertex
{
  "_key": "tt0111161",  // IMDB ID
  "_id": "movies/tt0111161",
  "title": "The Shawshank Redemption",
  "year": 1994,
  "genres": ["Drama", "Crime"],
  "rating": 9.3,
  "popularity": 98.5,
  "tmdb_id": 278,
  "vector_embedding": [...],  // 768-dim embedding for semantic search
  "quality_scores": {...},
  "scene_groups": ["..."]
}

// Actor Vertex
{
  "_key": "nm0000151",
  "_id": "actors/nm0000151",
  "name": "Morgan Freeman",
  "popularity": 95.2,
  "known_for": ["Drama", "Thriller"],
  "vector_embedding": [...]
}

// User Vertex
{
  "_key": "user_123",
  "_id": "users/user_123",
  "plex_id": "...",
  "preferences": {
    "genres": {"Drama": 0.8, "Action": 0.6},
    "decades": {"1990s": 0.7, "2000s": 0.9},
    "min_rating": 7.0
  },
  "taste_vector": [...]  // Learned taste embedding
}

// Genre Vertex
{
  "_key": "drama",
  "_id": "genres/drama",
  "name": "Drama",
  "parent": "genres/narrative"
}
```

#### Edges (Relationships)
```javascript
// acted_in Edge
{
  "_from": "actors/nm0000151",
  "_to": "movies/tt0111161",
  "role": "Ellis Boyd 'Red' Redding",
  "billing_order": 2,
  "screen_time": 85  // minutes
}

// directed Edge
{
  "_from": "directors/nm0001104",
  "_to": "movies/tt0111161",
  "primary": true
}

// watched Edge
{
  "_from": "users/user_123",
  "_to": "movies/tt0111161",
  "timestamp": "2024-01-15T20:30:00Z",
  "rating": 9,
  "completion": 100,  // percentage
  "device": "plex_tv"
}

// similar_to Edge (ML-generated)
{
  "_from": "movies/tt0111161",
  "_to": "movies/tt0068646",
  "similarity_score": 0.89,
  "factors": ["genre", "era", "themes", "cast"],
  "algorithm": "hybrid_v2"
}

// recommended Edge
{
  "_from": "movies/tt0111161",
  "_to": "users/user_123",
  "score": 0.92,
  "reason": "Based on your love for drama classics",
  "algorithm": "collaborative_content_hybrid",
  "timestamp": "2024-01-20T10:00:00Z"
}
```

### AQL Query Examples

#### Find Similar Movies Through Actor Relationships
```aql
// Find movies with shared actors (2+ in common)
FOR movie IN movies
  FILTER movie._key == @movie_id
  LET actors = (
    FOR v, e IN 1..1 INBOUND movie acted_in
      RETURN v._key
  )
  FOR other_movie IN movies
    FILTER other_movie._key != movie._key
    LET other_actors = (
      FOR v, e IN 1..1 INBOUND other_movie acted_in
        RETURN v._key
    )
    LET shared = LENGTH(INTERSECTION(actors, other_actors))
    FILTER shared >= 2
    SORT shared DESC
    LIMIT 10
    RETURN {
      movie: other_movie.title,
      shared_actors: shared,
      score: shared / LENGTH(actors)
    }
```

#### Graph Traversal for Recommendations
```aql
// Multi-hop traversal for deep recommendations
FOR user IN users
  FILTER user._key == @user_id
  // Get watched movies
  LET watched = (
    FOR v, e IN 1..1 OUTBOUND user watched
      RETURN v._key
  )
  // Find related content through multiple paths
  FOR movie IN movies
    FILTER movie._key NOT IN watched
    LET paths = (
      FOR v, e, p IN 1..3 ANY user 
        watched, acted_in, directed, similar_to, belongs_to
        OPTIONS {uniqueVertices: 'global'}
        FILTER v._id == movie._id
        RETURN {path: p, weight: 1 / LENGTH(p.edges)}
    )
    FILTER LENGTH(paths) > 0
    LET score = SUM(paths[*].weight)
    SORT score DESC
    LIMIT 20
    RETURN {
      movie: movie,
      recommendation_score: score,
      paths: LENGTH(paths),
      reasoning: paths[0].path
    }
```

#### Vector + Graph Hybrid Search (2024 Feature)
```aql
// Combine semantic similarity with graph relationships
LET vector_similar = (
  FOR movie IN movies
    LET similarity = COSINE_SIMILARITY(movie.vector_embedding, @user_taste_vector)
    FILTER similarity > 0.7
    SORT similarity DESC
    LIMIT 50
    RETURN {movie: movie, vector_score: similarity}
)

FOR item IN vector_similar
  LET graph_score = FIRST(
    FOR v, e, p IN 1..2 ANY item.movie
      acted_in, directed, similar_to
      OPTIONS {uniqueVertices: 'path'}
      FILTER v._key IN @user_watched_movies
      COLLECT AGGREGATE score = AVG(1 / LENGTH(p.edges))
      RETURN score
  )
  LET final_score = (item.vector_score * 0.6) + (graph_score * 0.4)
  SORT final_score DESC
  LIMIT 10
  RETURN {
    movie: item.movie,
    recommendation_score: final_score,
    vector_component: item.vector_score,
    graph_component: graph_score
  }
```

---

## 💡 Implementation Options

### Option 1: ArangoDB-Centric Graph Intelligence System 🎯 **RECOMMENDED**
**Approach**: Full ArangoDB integration with multi-model capabilities
- **Architecture**:
  - ArangoDB as primary graph store + vector database
  - PostgreSQL for transactional data only
  - Rust crate for ArangoDB integration using `arangors`
  - Graph algorithms implemented in AQL + Rust
- **Changes**:
  - New `radarr-graph` crate for ArangoDB operations
  - Migration scripts to populate initial graph
  - AQL stored procedures for complex traversals
  - Vector embedding pipeline for semantic search
- **Pros**:
  - Native graph operations with excellent performance
  - Multi-model flexibility (document + graph + vector)
  - Horizontal scaling with SmartGraphs
  - Built-in graph algorithms and traversals
  - 2024 vector search capabilities
- **Cons**:
  - Additional database to manage
  - Learning curve for AQL
  - Initial data migration complexity
- **Risk**: Medium
- **Verification**: 
  ```bash
  cargo test -p radarr-graph
  arangosh --server.endpoint http://localhost:8529 --javascript.execute tests/graph_verification.js
  ```

### Option 2: Hybrid ML with External Services
**Approach**: Combine ArangoDB graphs with external ML APIs
- **Architecture**:
  - ArangoDB for relationship storage
  - External ML services (Hugging Face, OpenAI) for embeddings
  - Redis for caching predictions
  - Federated learning for privacy
- **Changes**:
  - Integration layer for ML services
  - Embedding generation pipeline
  - Cache warming strategies
  - Privacy-preserving aggregation
- **Pros**:
  - Best-in-class ML models
  - No training infrastructure needed
  - Quick to deploy
- **Cons**:
  - API costs and rate limits
  - Privacy concerns
  - Network latency
- **Risk**: Low-Medium
- **Verification**: 
  ```bash
  cargo test test_ml_integration
  curl -X POST /api/v3/recommendations/generate
  ```

### Option 3: Reinforcement Learning with Graph Context
**Approach**: DDPG algorithm using graph features as state representation
- **Architecture**:
  - ArangoDB provides state features via graph traversal
  - Actor-Critic networks in Rust using Candle
  - Online learning from user feedback
  - Experience replay buffer in Redis
- **Changes**:
  - New `radarr-rl` crate with neural networks
  - Reward system based on user interactions
  - Training pipeline with checkpointing
  - A/B testing framework
- **Pros**:
  - Continuously improving recommendations
  - Handles exploration vs exploitation
  - Personalized per-user models
- **Cons**:
  - High complexity
  - Requires significant compute
  - Difficult to debug
- **Risk**: High
- **Verification**: 
  ```bash
  cargo bench rl_training_speed
  cargo test test_reward_calculation
  ```

### Option 4: Knowledge Graph with Reasoning
**Approach**: Ontology-based reasoning over movie knowledge graph
- **Architecture**:
  - ArangoDB stores RDF-like triples
  - OWL ontology for movie domain
  - SPARQL-to-AQL translation layer
  - Rule-based inference engine
- **Changes**:
  - Ontology definition for movie domain
  - Reasoning engine implementation
  - SPARQL compatibility layer
  - Explanation generation system
- **Pros**:
  - Explainable recommendations
  - Rich semantic relationships
  - Standards-based (RDF/OWL)
- **Cons**:
  - Complex ontology management
  - Performance overhead
  - Limited learning capability
- **Risk**: Medium-High
- **Verification**: 
  ```bash
  cargo test test_ontology_reasoning
  cargo run --bin verify-knowledge-graph
  ```

### Option 5: Collaborative Filtering with Graph Augmentation
**Approach**: Traditional CF enhanced with graph-derived features
- **Architecture**:
  - User-item matrix in PostgreSQL
  - Graph features from ArangoDB
  - Matrix factorization with graph regularization
  - Spark for distributed computation
- **Changes**:
  - Spark integration for large-scale CF
  - Feature extraction pipeline from graph
  - Hybrid scoring system
  - Batch prediction generation
- **Pros**:
  - Proven CF algorithms
  - Scalable with Spark
  - Good baseline performance
- **Cons**:
  - Cold start problem remains
  - Requires Spark infrastructure
  - Batch processing delays
- **Risk**: Low-Medium
- **Verification**: 
  ```bash
  cargo test test_collaborative_filtering
  spark-submit --class MovieRecommender target/recommender.jar
  ```

---

## 🚀 Plex Integration Strategy (Realistic Approach)

### Reality Check
Plex doesn't support true "placeholder" entries. We need to work within Plex's constraints while providing a seamless recommendation experience.

### Implementation Options

#### Option A: Dedicated Recommendations Library (Recommended) 📚
Create a separate Plex library specifically for recommendations using minimal dummy files:

##### Setup Requirements
1. Create dedicated directory: `/media/plex/Recommendations/`
2. Add as new Plex library (type: Movies, agent: Personal Media)
3. Configure library to use Local Media Assets as primary agent
4. Disable automatic metadata refresh to preserve custom data

##### Complete Implementation

```rust
// Rust implementation for Radarr MVP
pub struct RecommendationLibraryManager {
    plex_client: PlexClient,
    arango_client: ArangoClient,
    ffmpeg: FFmpegWrapper,
    library_path: PathBuf,
}

impl RecommendationLibraryManager {
    pub fn new(config: &Config) -> Self {
        Self {
            plex_client: PlexClient::new(&config.plex_url, &config.plex_token),
            arango_client: ArangoClient::new(&config.arango_url),
            ffmpeg: FFmpegWrapper::new(),
            library_path: PathBuf::from("/media/plex/Recommendations"),
        }
    }

    /// Create minimal video files for recommendations
    pub async fn create_recommendation_entries(&self, recommendations: Vec<MovieRecommendation>) -> Result<()> {
        // Ensure directory exists
        fs::create_dir_all(&self.library_path).await?;
        
        // Batch create all files first
        let mut created_files = Vec::new();
        
        for rec in recommendations {
            // Sanitize filename
            let safe_title = sanitize_filename(&rec.title);
            let video_filename = format!("{} ({}) [AI-REC-{}].mp4", 
                safe_title, 
                rec.year,
                (rec.score * 100.0) as u32
            );
            let video_path = self.library_path.join(&video_filename);
            
            // Generate minimal video (1 second, 320x240, ~10KB)
            self.create_minimal_video(&video_path).await?;
            
            // Create NFO file for local metadata
            self.create_nfo_file(&rec, &video_path).await?;
            
            // Generate poster with AI badge
            let poster_path = video_path.with_extension("jpg");
            self.generate_recommendation_poster(&rec, &poster_path).await?;
            
            created_files.push((rec, video_path));
        }
        
        // Trigger single library scan
        self.plex_client.scan_library_section("Recommendations").await?;
        
        // Wait for scan completion
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Update metadata via API
        for (rec, path) in created_files {
            self.update_plex_metadata(&rec, &path).await?;
            self.track_in_arango(&rec).await?;
        }
        
        Ok(())
    }
    
    async fn create_minimal_video(&self, path: &Path) -> Result<()> {
        // Create 1-second black video with minimal encoding
        let output = Command::new("ffmpeg")
            .args(&[
                "-f", "lavfi",
                "-i", "color=c=black:s=320x240:d=1:r=1",  // 1fps, 1 second
                "-vcodec", "libx264",
                "-preset", "ultrafast",
                "-crf", "51",  // Lowest quality for smallest size
                "-pix_fmt", "yuv420p",
                "-movflags", "+faststart",  // Optimize for streaming
                "-y",
                path.to_str().unwrap()
            ])
            .output()
            .await?;
            
        if !output.status.success() {
            return Err(anyhow!("FFmpeg failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(())
    }
    
    async fn create_nfo_file(&self, rec: &MovieRecommendation, video_path: &Path) -> Result<()> {
        let nfo_path = video_path.with_extension("nfo");
        let nfo_content = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<movie>
    <title>{}</title>
    <year>{}</year>
    <plot>🤖 AI RECOMMENDATION

Match Score: {}%
Confidence: {}
Algorithm: {}

REASONING:
{}

SIMILAR TO:
{}

This is a recommendation placeholder. Add to watchlist to download the actual movie.</plot>
    <rating>{}</rating>
    <mpaa>Not Rated</mpaa>
    <tmdbid>{}</tmdbid>
    <imdbid>{}</imdbid>
    <genre>AI Recommendation</genre>
    <tag>ai-recommendation</tag>
    <tag>confidence-{}</tag>
    <tag>not-downloaded</tag>
</movie>"#,
            rec.title,
            rec.year,
            rec.score * 100.0,
            rec.confidence_tier,
            rec.algorithm_used,
            rec.reasoning,
            rec.similar_movies.join(", "),
            rec.predicted_rating,
            rec.tmdb_id,
            rec.imdb_id,
            rec.confidence_tier
        );
        
        fs::write(nfo_path, nfo_content).await?;
        Ok(())
    }
    
    async fn update_plex_metadata(&self, rec: &MovieRecommendation, path: &Path) -> Result<()> {
        // Use Plex HTTP API for metadata updates
        let filename = path.file_name().unwrap().to_str().unwrap();
        
        // Search for the item in the library
        let search_url = format!(
            "{}/library/sections/{}/all?X-Plex-Token={}&title={}",
            self.plex_client.base_url,
            self.get_section_id("Recommendations").await?,
            self.plex_client.token,
            urlencoding::encode(filename)
        );
        
        let item_key = self.find_item_key(&search_url).await?;
        
        // Update metadata
        let update_url = format!(
            "{}/library/metadata/{}?X-Plex-Token={}",
            self.plex_client.base_url,
            item_key,
            self.plex_client.token
        );
        
        let params = [
            ("summary.value", format!(
                "🤖 AI RECOMMENDATION - {}% Match\n\n{}\n\nWhy recommended: {}\n\n⚠️ Add to watchlist to download",
                rec.score * 100.0,
                rec.overview,
                rec.reasoning
            )),
            ("rating.value", rec.predicted_rating.to_string()),
            ("tagline.value", format!("AI Pick: {}% confidence", rec.score * 100.0)),
            ("contentRating.value", "AI-REC".to_string()),
        ];
        
        self.plex_client.put(&update_url, &params).await?;
        
        // Upload poster if available
        if let Some(poster_data) = &rec.poster_data {
            self.upload_poster(item_key, poster_data).await?;
        }
        
        Ok(())
    }
}

// Python helper script for Plex interaction (alternative approach)
```

```python
#!/usr/bin/env python3
# plex_recommendation_manager.py

from plexapi.server import PlexServer
from plexapi.exceptions import NotFound
from PIL import Image, ImageDraw, ImageFont
import subprocess
import json
import os
from pathlib import Path
from datetime import datetime

class PlexRecommendationManager:
    def __init__(self, plex_url, plex_token, library_path="/media/plex/Recommendations"):
        self.plex = PlexServer(plex_url, plex_token)
        self.library_path = Path(library_path)
        self.library_path.mkdir(parents=True, exist_ok=True)
        
        # Ensure Recommendations library exists
        self.ensure_library_exists()
    
    def ensure_library_exists(self):
        """Create Recommendations library if it doesn't exist"""
        try:
            self.rec_library = self.plex.library.section('Recommendations')
        except NotFound:
            # Create new library
            self.plex.library.add(
                name='Recommendations',
                type='movie',
                agent='com.plexapp.agents.none',  # Use Personal Media
                scanner='Plex Movie Scanner',
                location=str(self.library_path)
            )
            self.rec_library = self.plex.library.section('Recommendations')
    
    def create_recommendation_batch(self, recommendations):
        """Create a batch of recommendation entries"""
        created_items = []
        
        for rec in recommendations:
            # Create minimal video file
            video_path = self.create_minimal_video(rec)
            
            # Create poster with AI badge
            poster_path = self.create_ai_poster(rec)
            
            # Create subtitle file with recommendation details
            self.create_info_subtitle(rec, video_path)
            
            created_items.append((rec, video_path))
        
        # Scan library once for all new items
        self.rec_library.scan()
        
        # Wait for scan to complete
        import time
        time.sleep(5)
        
        # Update metadata for each item
        for rec, video_path in created_items:
            self.update_metadata(rec, video_path)
    
    def create_minimal_video(self, rec):
        """Create a minimal 1-second video file"""
        safe_title = "".join(c for c in rec['title'] if c.isalnum() or c in (' ', '-', '_')).rstrip()
        filename = f"{safe_title} ({rec['year']}) [AI-{int(rec['score']*100)}].mp4"
        video_path = self.library_path / filename
        
        # FFmpeg command for minimal video
        cmd = [
            'ffmpeg', '-y',
            '-f', 'lavfi',
            '-i', f"color=c=black:s=320x240:d=1:r=1",
            '-vcodec', 'libx264',
            '-preset', 'ultrafast',
            '-crf', '51',
            '-pix_fmt', 'yuv420p',
            '-metadata', f"title={rec['title']} (AI Recommendation)",
            '-metadata', f"comment=AI Score: {rec['score']*100:.0f}%",
            str(video_path)
        ]
        
        subprocess.run(cmd, check=True, capture_output=True)
        return video_path
    
    def create_ai_poster(self, rec):
        """Generate poster with AI recommendation badge"""
        poster_path = self.library_path / f"{rec['tmdb_id']}_poster.jpg"
        
        # Download original poster
        import requests
        if rec.get('poster_url'):
            response = requests.get(rec['poster_url'])
            with open(poster_path, 'wb') as f:
                f.write(response.content)
            
            # Add AI badge overlay
            img = Image.open(poster_path)
            draw = ImageDraw.Draw(img)
            
            # Add semi-transparent overlay at top
            overlay = Image.new('RGBA', img.size, (0,0,0,0))
            overlay_draw = ImageDraw.Draw(overlay)
            overlay_draw.rectangle([(0,0), (img.width, 100)], fill=(0,0,0,180))
            
            # Composite overlay
            img = Image.alpha_composite(img.convert('RGBA'), overlay)
            draw = ImageDraw.Draw(img)
            
            # Add text
            try:
                font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 24)
                small_font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 18)
            except:
                font = ImageFont.load_default()
                small_font = font
            
            draw.text((10, 10), "🤖 AI PICK", font=font, fill=(255,255,255))
            draw.text((10, 45), f"{rec['score']*100:.0f}% Match", font=small_font, fill=(255,255,255))
            draw.text((10, 70), rec['confidence_tier'].upper(), font=small_font, fill=(255,200,0))
            
            # Save modified poster
            img.convert('RGB').save(poster_path, quality=90)
        
        return poster_path
    
    def update_metadata(self, rec, video_path):
        """Update Plex metadata for recommendation"""
        try:
            # Find the item in Plex
            video_name = video_path.stem
            items = self.rec_library.search(title=video_name)
            
            if not items:
                print(f"Item not found: {video_name}")
                return
            
            item = items[0]
            
            # Update metadata fields
            item.editTitle(f"🤖 {rec['title']}")
            item.editSummary(
                f"AI RECOMMENDATION\n"
                f"═══════════════════\n"
                f"Match Score: {rec['score']*100:.0f}%\n"
                f"Confidence: {rec['confidence_tier']}\n"
                f"Algorithm: {rec['algorithm']}\n\n"
                f"REASONING:\n{rec['reasoning']}\n\n"
                f"SIMILAR TO:\n{', '.join(rec['similar_movies'])}\n\n"
                f"⚠️ This is a recommendation placeholder.\n"
                f"Add to watchlist to download the actual movie."
            )
            
            # Set content rating to identify as recommendation
            item.editContentRating("AI-REC")
            
            # Add collections
            item.addCollection("AI Recommendations")
            item.addCollection(f"AI {rec['confidence_tier'].title()} Confidence")
            
            # Add labels for filtering
            item.addLabel("ai-recommendation")
            item.addLabel(f"score-{int(rec['score']*100/10)*10}")  # Round to nearest 10
            item.addLabel(rec['algorithm'])
            
            # Upload poster if exists
            poster_path = self.library_path / f"{rec['tmdb_id']}_poster.jpg"
            if poster_path.exists():
                item.uploadPoster(filepath=str(poster_path))
            
            print(f"✅ Updated metadata for: {rec['title']}")
            
        except Exception as e:
            print(f"❌ Error updating {rec['title']}: {e}")
    
    def cleanup_converted(self):
        """Remove placeholders that have been downloaded"""
        for item in self.rec_library.all():
            if item.contentRating == "AI-REC":
                # Check if real version exists in main Movies library
                try:
                    main_library = self.plex.library.section('Movies')
                    # Search by TMDB ID in main library
                    real_movie = main_library.search(guid=f"tmdb://{item.guid.split('/')[-1]}")
                    if real_movie:
                        # Real movie exists, remove placeholder
                        item.delete()
                        # Also delete the file
                        video_files = item.media[0].parts[0].file
                        os.remove(video_files)
                        print(f"🧹 Cleaned up placeholder for: {item.title}")
                except:
                    pass

# Usage example
if __name__ == "__main__":
    manager = PlexRecommendationManager(
        plex_url="http://localhost:32400",
        plex_token="YOUR_PLEX_TOKEN"
    )
    
    # Example recommendations from ML system
    recommendations = [
        {
            'title': 'The Shawshank Redemption',
            'year': 1994,
            'tmdb_id': 278,
            'imdb_id': 'tt0111161',
            'score': 0.92,
            'confidence_tier': 'high',
            'algorithm': 'hybrid_collaborative_graph',
            'reasoning': 'Based on your love for character-driven dramas with themes of hope and redemption',
            'similar_movies': ['The Green Mile', 'The Pianist'],
            'poster_url': 'https://image.tmdb.org/t/p/w500/...',
            'overview': 'Two imprisoned men bond over years...'
        }
    ]
    
    manager.create_recommendation_batch(recommendations)
```
```

#### Option B: Plex Watchlist Direct Integration
Skip the library entirely and work directly with Plex's watchlist:

```rust
pub struct WatchlistRecommendationManager {
    plex_client: PlexClient,
    recommendation_api: RecommendationAPI,
}

impl WatchlistRecommendationManager {
    /// Add recommendations directly to Plex Discover/Watchlist
    pub async fn push_to_plex_discover(&self, user_id: &str) -> Result<()> {
        let recommendations = self.recommendation_api.get_recommendations(user_id).await?;
        
        for rec in recommendations {
            // Use Plex's official Discover/Watchlist API
            // This adds items that don't exist in library to a special "Discover" section
            self.plex_client
                .add_to_watchlist(rec.tmdb_id)
                .with_metadata(PlexMetadata {
                    custom_note: format!("AI: {}% match - {}", rec.score * 100.0, rec.reasoning),
                    predicted_rating: rec.predicted_rating,
                })
                .await?;
        }
        
        // Watchlistarr-rust automatically picks these up
        Ok(())
    }
}
```

#### Option C: Hybrid Web Dashboard + Plex Integration
Most flexible and user-friendly approach:

```rust
pub struct RecommendationDashboard {
    plex_client: PlexClient,
    radarr_client: RadarrClient,
    arango: ArangoClient,
}

impl RecommendationDashboard {
    /// Serve web dashboard at http://radarr:7878/recommendations
    pub async fn render_dashboard(&self, user_id: &str) -> Html {
        let recommendations = self.get_recommendations(user_id).await?;
        
        html! {
            <div class="recommendations-grid">
            { for recommendations.iter().map(|rec| self.render_recommendation_card(rec)) }
            </div>
        }
    }
    
    fn render_recommendation_card(&self, rec: &Recommendation) -> Html {
        let in_library = self.check_if_in_plex_library(&rec.tmdb_id).await?;
        let in_watchlist = self.check_if_in_watchlist(&rec.tmdb_id).await?;
        
        html! {
            <div class="recommendation-card">
                <img src={rec.poster_url} />
                <h3>{rec.title} ({rec.year})</h3>
                <div class="ai-score">{format!("{}% Match", rec.score * 100)}</div>
                <p class="reasoning">{rec.reasoning}</p>
                
                { if in_library {
                    html! { <span class="badge">Already in Library</span> }
                } else if in_watchlist {
                    html! { <span class="badge">In Watchlist (Downloading)</span> }
                } else {
                    html! {
                        <div class="actions">
                            <button onclick={add_to_watchlist(rec.tmdb_id)}>
                                Add to Plex Watchlist
                            </button>
                            <button onclick={download_now(rec.tmdb_id)}>
                                Download Now
                            </button>
                            <button onclick={not_interested(rec.id)}>
                                Not Interested
                            </button>
                        </div>
                    }
                }}
            </div>
        }
    }
}
```

### Realistic Workflow

1. **Discovery Phase**
   - ML system generates recommendations
   - Store in ArangoDB with relationships
   - Calculate confidence scores and reasoning

2. **Presentation Phase**
   - **Dashboard**: Show in Radarr web UI at `/recommendations`
   - **Plex Library**: Optional minimal files in "Recommendations" library
   - **API**: Expose via `/api/v3/recommendations` for third-party apps
   - **Notifications**: Send weekly digest of top recommendations

3. **Action Phase**
   - User clicks "Add to Watchlist" → Plex Watchlist API
   - Watchlistarr-rust detects addition → Triggers Radarr
   - User clicks "Download Now" → Direct Radarr search
   - User clicks "Not Interested" → Update ML model

4. **Feedback Loop**
   - Track user interactions (accepted/rejected)
   - Monitor viewing behavior for accepted recommendations
   - Adjust ML model weights based on feedback
   - Update graph relationships in ArangoDB

### Technical Implementation Details

```rust
// Minimal video generation for Plex library approach
pub async fn create_minimal_video(path: &str, duration_secs: u32) -> Result<()> {
    Command::new("ffmpeg")
        .args(&[
            "-f", "lavfi",
            "-i", &format!("color=c=black:s=320x240:d={}", duration_secs),
            "-vcodec", "libx264",
            "-pix_fmt", "yuv420p",
            "-preset", "ultrafast",
            "-y",
            path
        ])
        .status()
        .await?;
    Ok(())
}

// Poster generation with recommendation badge
pub async fn generate_recommendation_poster(
    original_poster_url: &str,
    confidence: f32
) -> Result<Vec<u8>> {
    let mut img = fetch_image(original_poster_url).await?;
    
    // Add semi-transparent overlay
    let overlay = create_gradient_overlay(img.dimensions());
    imageops::overlay(&mut img, &overlay, 0, 0);
    
    // Add text badges
    draw_text(&mut img, &format!("AI PICK"), 10, 10);
    draw_text(&mut img, &format!("{}% Match", confidence * 100.0), 10, 50);
    
    // Convert to bytes
    let mut buffer = Vec::new();
    img.write_to(&mut buffer, ImageOutputFormat::Jpeg(90))?;
    Ok(buffer)
}

---

## 📊 ML/AI Algorithm Details

### Hybrid Recommendation System

#### 1. Collaborative Filtering Component
```rust
pub struct CollaborativeFilter {
    user_embeddings: DMatrix<f32>,
    item_embeddings: DMatrix<f32>,
    regularization: f32,
}

impl CollaborativeFilter {
    pub fn train(&mut self, interactions: &UserItemMatrix) {
        // Alternating Least Squares (ALS)
        for _ in 0..self.iterations {
            // Fix items, solve for users
            self.update_user_embeddings(&interactions);
            // Fix users, solve for items  
            self.update_item_embeddings(&interactions);
        }
    }
    
    pub fn predict(&self, user_id: UserId, item_id: ItemId) -> f32 {
        let user_vec = self.user_embeddings.row(user_id);
        let item_vec = self.item_embeddings.row(item_id);
        user_vec.dot(&item_vec)
    }
}
```

#### 2. Content-Based Filtering Component
```rust
pub struct ContentBasedFilter {
    feature_extractor: FeatureExtractor,
    similarity_metric: SimilarityMetric,
}

impl ContentBasedFilter {
    pub async fn extract_features(&self, movie: &Movie) -> FeatureVector {
        let mut features = FeatureVector::new();
        
        // Genre features
        features.extend(self.encode_genres(&movie.genres));
        
        // Actor/Director features from graph
        let cast_features = self.extract_cast_features(movie).await?;
        features.extend(cast_features);
        
        // Textual features from synopsis
        let text_embedding = self.encode_text(&movie.synopsis).await?;
        features.extend(text_embedding);
        
        // Technical features (year, runtime, rating)
        features.extend(self.encode_metadata(movie));
        
        features
    }
    
    pub fn compute_similarity(&self, a: &FeatureVector, b: &FeatureVector) -> f32 {
        match self.similarity_metric {
            SimilarityMetric::Cosine => cosine_similarity(a, b),
            SimilarityMetric::Euclidean => 1.0 / (1.0 + euclidean_distance(a, b)),
            SimilarityMetric::Jaccard => jaccard_similarity(a, b),
        }
    }
}
```

#### 3. Graph-Based Features
```rust
pub struct GraphFeatureExtractor {
    arango: ArangoClient,
}

impl GraphFeatureExtractor {
    pub async fn extract_graph_features(&self, movie_id: &str) -> GraphFeatures {
        // PageRank score for movie importance
        let pagerank = self.compute_pagerank(movie_id).await?;
        
        // Community detection for genre clusters
        let community = self.detect_community(movie_id).await?;
        
        // Path-based features to popular movies
        let path_features = self.extract_path_features(movie_id).await?;
        
        // Centrality measures
        let centrality = self.compute_centrality(movie_id).await?;
        
        GraphFeatures {
            pagerank,
            community_id: community,
            avg_path_length: path_features.avg_length,
            betweenness_centrality: centrality.betweenness,
            degree_centrality: centrality.degree,
        }
    }
}
```

#### 4. Ensemble Model
```rust
pub struct HybridRecommender {
    collaborative: CollaborativeFilter,
    content_based: ContentBasedFilter,
    graph_extractor: GraphFeatureExtractor,
    ensemble_weights: EnsembleWeights,
}

impl HybridRecommender {
    pub async fn recommend(&self, user_id: UserId, n: usize) -> Vec<Recommendation> {
        // Get candidates from each component
        let cf_candidates = self.collaborative.get_candidates(user_id, n * 3);
        let cb_candidates = self.content_based.get_candidates(user_id, n * 3);
        let graph_candidates = self.graph_extractor.get_candidates(user_id, n * 3).await;
        
        // Merge and score
        let mut all_candidates = HashMap::new();
        
        for (movie_id, cf_score) in cf_candidates {
            let cb_score = cb_candidates.get(&movie_id).unwrap_or(&0.0);
            let graph_score = graph_candidates.get(&movie_id).unwrap_or(&0.0);
            
            let final_score = 
                self.ensemble_weights.collaborative * cf_score +
                self.ensemble_weights.content_based * cb_score +
                self.ensemble_weights.graph_based * graph_score;
            
            all_candidates.insert(movie_id, final_score);
        }
        
        // Sort and return top N
        let mut recommendations: Vec<_> = all_candidates.into_iter().collect();
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        recommendations.truncate(n);
        
        // Add explanations
        self.add_explanations(recommendations).await
    }
}
```

---

## 📈 Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)
**Goal**: Establish ArangoDB infrastructure and basic graph model

#### Week 1-2: ArangoDB Integration
- [ ] Install and configure ArangoDB cluster
- [ ] Create `radarr-graph` Rust crate
- [ ] Implement `arangors` client wrapper
- [ ] Design initial graph schema
- [ ] Create migration scripts from PostgreSQL

#### Week 3-4: Graph Population
- [ ] Import movies, actors, directors from TMDB
- [ ] Build initial relationship edges
- [ ] Create user vertices from existing data
- [ ] Implement basic AQL queries
- [ ] Performance testing and optimization

### Phase 2: ML Components (Weeks 5-8)
**Goal**: Build recommendation algorithm components

#### Week 5-6: Collaborative Filtering
- [ ] Implement ALS algorithm in Rust
- [ ] Create user-item interaction matrix
- [ ] Build training pipeline
- [ ] Add cross-validation framework
- [ ] Optimize for sparse matrices

#### Week 7-8: Content-Based Filtering
- [ ] Feature extraction pipeline
- [ ] Text embedding generation (synopsis, reviews)
- [ ] Similarity computation algorithms
- [ ] Integration with ArangoDB features
- [ ] Caching layer for embeddings

### Phase 3: Graph Intelligence (Weeks 9-12)
**Goal**: Advanced graph algorithms and vector search

#### Week 9-10: Graph Algorithms
- [ ] PageRank implementation in AQL
- [ ] Community detection algorithms
- [ ] Path-based similarity measures
- [ ] Centrality calculations
- [ ] Graph traversal optimization

#### Week 11-12: Vector Search Integration
- [ ] Generate movie embeddings using BERT/transformers
- [ ] Store vectors in ArangoDB (FAISS backend)
- [ ] Implement semantic search endpoints
- [ ] Hybrid search (vector + graph)
- [ ] Performance benchmarking

### Phase 4: Plex Integration (Weeks 13-16)
**Goal**: Seamless Plex placeholder system

#### Week 13-14: Placeholder Infrastructure
- [ ] Placeholder file generation system
- [ ] Plex API integration for virtual entries
- [ ] Metadata management for placeholders
- [ ] Visual indicators (posters, badges)
- [ ] Cleanup and lifecycle management

#### Week 15-16: Watchlist Integration
- [ ] Watchlistarr-rust modifications
- [ ] Placeholder detection in watchlist
- [ ] Conversion pipeline (placeholder → real media)
- [ ] Feedback loop to ML system
- [ ] End-to-end testing

### Phase 5: Production Readiness (Weeks 17-20)
**Goal**: Polish, optimize, and deploy

#### Week 17-18: Optimization
- [ ] Query performance tuning
- [ ] Caching strategies
- [ ] Batch processing pipelines
- [ ] Resource usage optimization
- [ ] Load testing

#### Week 19-20: Deployment
- [ ] Kubernetes manifests for ArangoDB
- [ ] Monitoring and alerting setup
- [ ] Documentation and API specs
- [ ] User guides and tutorials
- [ ] Production deployment

---

## 🎯 Performance Targets

### Query Performance
- **Graph Traversal (3-depth)**: <50ms p95
- **Recommendation Generation**: <100ms p95
- **Vector Search (100K items)**: <20ms p95
- **Hybrid Queries**: <150ms p95

### Scale Metrics
- **Movies**: 500K+ vertices
- **Relationships**: 10M+ edges
- **Users**: 10K+ concurrent
- **Recommendations**: 1M+ per day

### Quality Metrics
- **Precision@10**: >0.7
- **Recall@10**: >0.5
- **NDCG**: >0.8
- **User Satisfaction**: >85%

---

## 🔧 Technology Stack

### Core Technologies
- **Graph Database**: ArangoDB 3.12+ (multi-model)
- **ML Framework**: Candle (Rust-native neural networks)
- **Linear Algebra**: nalgebra, ndarray
- **Async Runtime**: Tokio
- **API Layer**: Axum + GraphQL

### Rust Dependencies
```toml
[dependencies]
# Graph Database
arangors = "0.5"
serde_json = "1.0"

# Machine Learning
candle-core = "0.3"
candle-nn = "0.3"
nalgebra = "0.32"
ndarray = "0.15"

# Feature Engineering
tokenizers = "0.15"
rust-bert = "0.21"

# Async
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# API
axum = "0.7"
async-graphql = "7.0"
async-graphql-axum = "7.0"
```

---

## 🚨 Risk Mitigation

### Technical Risks
1. **ArangoDB Learning Curve**
   - Mitigation: Team training, hire consultant
   - Fallback: Use Neo4j if needed

2. **ML Model Performance**
   - Mitigation: Start simple, iterate
   - Fallback: External API services

3. **Plex API Limitations**
   - Mitigation: Work with Plex team
   - Fallback: Separate recommendation UI

### Operational Risks
1. **Data Privacy**
   - Mitigation: Federated learning, local models
   - Compliance: GDPR/CCPA adherence

2. **Scalability**
   - Mitigation: Horizontal sharding, caching
   - Monitoring: Comprehensive metrics

---

## 📋 Success Criteria

### Minimum Viable Feature Set
- [x] Basic graph model with movies and relationships
- [x] Simple collaborative filtering
- [x] Content-based recommendations
- [x] API endpoints for recommendations
- [ ] Basic Plex placeholder system

### Full Feature Set
- [ ] Complete graph with all relationships
- [ ] Hybrid ML algorithms
- [ ] Vector search integration
- [ ] Full Plex placeholder lifecycle
- [ ] Learning from user feedback
- [ ] Explanation generation
- [ ] A/B testing framework

---

## 🎬 Conclusion

This advanced feature set transforms Radarr from a media management tool into an **intelligent content discovery platform**. By leveraging ArangoDB's graph capabilities combined with modern ML techniques, we can provide users with highly personalized, explainable recommendations that seamlessly integrate with their existing Plex setup.

The phased approach allows for incremental value delivery while building toward a sophisticated recommendation system that rivals commercial platforms. The use of graph databases provides unique advantages in understanding complex relationships that traditional recommendation systems miss.

### Next Steps
1. **Proof of Concept**: Build minimal ArangoDB integration (1 week)
2. **Prototype**: Basic recommendations with 1000 movies (2 weeks)
3. **User Testing**: Deploy to beta users for feedback (ongoing)
4. **Iterate**: Refine based on real-world usage

### Investment Required
- **Development Time**: 20 weeks (800 hours)
- **Infrastructure**: ArangoDB cluster, ML compute resources
- **Team**: 1-2 senior engineers, 1 ML specialist
- **Estimated Cost**: $120,000 - $180,000

The combination of graph intelligence, machine learning, and seamless UX integration positions this as a unique and valuable addition to the Radarr ecosystem.