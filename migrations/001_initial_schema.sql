-- Initial PostgreSQL-only schema for Radarr MVP
-- Performance-optimized design with JSONB metadata and full-text search

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Movies table - Core entity for movie management
CREATE TABLE movies (
    -- Core identification
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tmdb_id INTEGER UNIQUE NOT NULL,
    imdb_id TEXT,
    
    -- Basic information
    title TEXT NOT NULL,
    original_title TEXT,
    year INTEGER,
    runtime INTEGER, -- minutes
    
    -- Status and monitoring
    status TEXT NOT NULL DEFAULT 'announced',
    -- announced, in_production, post_production, released, cancelled
    monitored BOOLEAN NOT NULL DEFAULT true,
    
    -- Quality and availability
    quality_profile_id INTEGER,
    minimum_availability TEXT DEFAULT 'released',
    -- announced, in_cinemas, released, predb
    
    -- File information
    has_file BOOLEAN NOT NULL DEFAULT false,
    movie_file_id UUID,
    
    -- Flexible metadata storage (replaces EdgeDB graph features)
    metadata JSONB NOT NULL DEFAULT '{}',
    -- Contains: overview, poster_path, backdrop_path, vote_average, 
    --          vote_count, popularity, adult, video, original_language,
    --          production_companies, production_countries, spoken_languages,
    --          belongs_to_collection, genres, keywords, etc.
    
    -- Alternative titles for search
    alternative_titles JSONB NOT NULL DEFAULT '[]',
    -- Array of title objects: [{"title": "Alt Title", "language": "en"}]
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_search_time TIMESTAMPTZ,
    last_info_sync TIMESTAMPTZ
);

-- Quality profiles table
CREATE TABLE quality_profiles (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    
    -- Quality configuration
    cutoff_quality_id INTEGER NOT NULL,
    upgrade_allowed BOOLEAN NOT NULL DEFAULT true,
    
    -- Quality items configuration
    items JSONB NOT NULL,
    -- Array of quality items with allowed/preferred settings
    
    -- Language preferences
    language TEXT NOT NULL DEFAULT 'english',
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Download clients table
CREATE TABLE download_clients (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    implementation TEXT NOT NULL, -- transmission, qbittorrent, etc.
    
    -- Client configuration
    settings JSONB NOT NULL,
    -- Contains: host, port, username, password, category, etc.
    
    -- Client behavior
    enabled BOOLEAN NOT NULL DEFAULT true,
    priority INTEGER NOT NULL DEFAULT 1,
    remove_completed_downloads BOOLEAN NOT NULL DEFAULT true,
    remove_failed_downloads BOOLEAN NOT NULL DEFAULT true,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexers table
CREATE TABLE indexers (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    implementation TEXT NOT NULL, -- prowlarr, jackett, etc.
    
    -- Indexer configuration
    settings JSONB NOT NULL,
    -- Contains: base_url, api_key, categories, capabilities, etc.
    
    -- Indexer behavior
    enabled BOOLEAN NOT NULL DEFAULT true,
    priority INTEGER NOT NULL DEFAULT 25,
    enable_rss BOOLEAN NOT NULL DEFAULT true,
    enable_automatic_search BOOLEAN NOT NULL DEFAULT true,
    enable_interactive_search BOOLEAN NOT NULL DEFAULT true,
    
    -- Rate limiting
    download_client_id INTEGER REFERENCES download_clients(id),
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Downloads table
CREATE TABLE downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movie_id UUID NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    download_client_id INTEGER NOT NULL REFERENCES download_clients(id),
    indexer_id INTEGER REFERENCES indexers(id),
    
    -- Download identification
    download_id TEXT NOT NULL, -- client-specific ID
    title TEXT NOT NULL,
    category TEXT,
    
    -- Download status
    status TEXT NOT NULL, -- downloading, completed, failed, removed
    size_bytes BIGINT,
    size_left BIGINT,
    
    -- Quality information
    quality JSONB NOT NULL,
    
    -- Progress tracking
    download_time TIMESTAMPTZ,
    completion_time TIMESTAMPTZ,
    error_message TEXT,
    
    -- Import information
    imported BOOLEAN NOT NULL DEFAULT false,
    import_time TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Movie files table
CREATE TABLE movie_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movie_id UUID NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    
    -- File location
    relative_path TEXT NOT NULL, -- relative to root folder
    size_bytes BIGINT NOT NULL,
    
    -- Quality information
    quality JSONB NOT NULL,
    -- Contains: quality_name, resolution, source, codec, etc.
    
    -- Media information
    media_info JSONB,
    -- Contains: duration, bitrate, audio_channels, subtitle_languages, etc.
    
    -- File metadata
    date_added TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_write_time TIMESTAMPTZ,
    checksum TEXT, -- for integrity verification
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Commands table for background tasks
CREATE TABLE commands (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    command_name TEXT NOT NULL,
    
    -- Command payload
    body JSONB NOT NULL DEFAULT '{}',
    
    -- Execution tracking
    status TEXT NOT NULL DEFAULT 'queued', -- queued, started, completed, failed, cancelled
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    -- Progress and results
    progress JSONB,
    result JSONB,
    error_message TEXT,
    
    -- Execution context
    trigger TEXT, -- manual, scheduled, automatic
    priority INTEGER NOT NULL DEFAULT 0,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Logs table for system logging
CREATE TABLE logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level TEXT NOT NULL, -- trace, debug, info, warn, error, fatal
    logger TEXT NOT NULL,
    message TEXT NOT NULL,
    
    -- Exception details
    exception_type TEXT,
    exception_message TEXT,
    stack_trace TEXT,
    
    -- Additional context
    context JSONB,
    
    -- Performance tracking
    execution_time_ms INTEGER
);

-- Basic indexes for performance
CREATE UNIQUE INDEX idx_movies_tmdb_id ON movies (tmdb_id);
CREATE INDEX idx_movies_imdb_id ON movies (imdb_id) WHERE imdb_id IS NOT NULL;
CREATE INDEX idx_movies_status ON movies (status);
CREATE INDEX idx_movies_monitored ON movies (monitored) WHERE monitored = true;
CREATE INDEX idx_movies_year ON movies (year);
CREATE INDEX idx_movies_title ON movies (title);

-- File relationships
CREATE INDEX idx_movie_files_movie_id ON movie_files (movie_id);
CREATE INDEX idx_downloads_movie_id ON downloads (movie_id);
CREATE INDEX idx_downloads_status ON downloads (status);
CREATE INDEX idx_downloads_client_id ON downloads (download_client_id);

-- Quality and indexer indexes
CREATE INDEX idx_quality_profiles_name ON quality_profiles (name);
CREATE INDEX idx_indexers_enabled ON indexers (enabled) WHERE enabled = true;
CREATE INDEX idx_download_clients_enabled ON download_clients (enabled) WHERE enabled = true;

-- Command and log indexes
CREATE INDEX idx_commands_status ON commands (status);
CREATE INDEX idx_commands_queued_at ON commands (queued_at);
CREATE INDEX idx_logs_timestamp ON logs (timestamp);
CREATE INDEX idx_logs_level ON logs (level);

-- Foreign key constraint for movies -> quality_profiles
ALTER TABLE movies ADD CONSTRAINT fk_movies_quality_profile_id 
    FOREIGN KEY (quality_profile_id) REFERENCES quality_profiles(id);

-- Foreign key constraint for movies -> movie_files
ALTER TABLE movies ADD CONSTRAINT fk_movies_movie_file_id 
    FOREIGN KEY (movie_file_id) REFERENCES movie_files(id);