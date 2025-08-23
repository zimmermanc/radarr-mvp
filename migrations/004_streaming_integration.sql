-- Migration: 004_streaming_integration.sql
-- Purpose: Add streaming service integration tables for TMDB + Trakt + Watchmode
-- Created: 2025-08-23

-- Streaming cache table for aggressive API response caching
CREATE TABLE IF NOT EXISTS streaming_cache (
    cache_key VARCHAR(255) PRIMARY KEY,
    data JSONB NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient expiration queries
CREATE INDEX idx_streaming_cache_expires ON streaming_cache(expires_at);

-- TMDB to Watchmode ID mapping table
CREATE TABLE IF NOT EXISTS streaming_id_mappings (
    tmdb_id INTEGER PRIMARY KEY,
    watchmode_id INTEGER,
    media_type VARCHAR(20) NOT NULL CHECK (media_type IN ('movie', 'tv')),
    last_verified TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for Watchmode lookups
CREATE INDEX idx_streaming_id_mappings_watchmode ON streaming_id_mappings(watchmode_id);
CREATE INDEX idx_streaming_id_mappings_type ON streaming_id_mappings(media_type);

-- Trending entries table for aggregated trending data
CREATE TABLE IF NOT EXISTS trending_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tmdb_id INTEGER NOT NULL,
    media_type VARCHAR(20) NOT NULL CHECK (media_type IN ('movie', 'tv')),
    title VARCHAR(500) NOT NULL,
    release_date DATE,
    poster_path VARCHAR(255),
    backdrop_path VARCHAR(255),
    overview TEXT,
    source VARCHAR(50) NOT NULL CHECK (source IN ('tmdb', 'trakt', 'aggregated')),
    time_window VARCHAR(10) NOT NULL CHECK (time_window IN ('day', 'week')),
    rank INTEGER,
    score DECIMAL(5,2),
    vote_average DECIMAL(3,1),
    vote_count INTEGER,
    popularity DECIMAL(10,2),
    fetched_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tmdb_id, media_type, source, time_window)
);

-- Indexes for trending queries
CREATE INDEX idx_trending_entries_lookup ON trending_entries(media_type, source, time_window, expires_at);
CREATE INDEX idx_trending_entries_tmdb ON trending_entries(tmdb_id, media_type);
CREATE INDEX idx_trending_entries_rank ON trending_entries(source, time_window, rank);

-- Streaming availability table
CREATE TABLE IF NOT EXISTS streaming_availability (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tmdb_id INTEGER NOT NULL,
    media_type VARCHAR(20) NOT NULL CHECK (media_type IN ('movie', 'tv')),
    region VARCHAR(2) DEFAULT 'US',
    service_name VARCHAR(100) NOT NULL,
    service_type VARCHAR(20) CHECK (service_type IN ('subscription', 'rent', 'buy', 'free', 'ads')),
    service_logo_url VARCHAR(500),
    deep_link VARCHAR(1000),
    price_amount DECIMAL(6,2),
    price_currency VARCHAR(3) DEFAULT 'USD',
    quality VARCHAR(20) CHECK (quality IN ('SD', 'HD', '4K', 'HDR')),
    leaving_date DATE,
    fetched_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tmdb_id, media_type, region, service_name, service_type)
);

-- Indexes for availability queries
CREATE INDEX idx_streaming_availability_lookup ON streaming_availability(tmdb_id, media_type, region);
CREATE INDEX idx_streaming_availability_service ON streaming_availability(service_name, region);
CREATE INDEX idx_streaming_availability_expires ON streaming_availability(expires_at);

-- OAuth tokens table for Trakt authentication
CREATE TABLE IF NOT EXISTS oauth_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service VARCHAR(50) NOT NULL UNIQUE,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    token_type VARCHAR(50) DEFAULT 'Bearer',
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    scope TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for token lookups
CREATE INDEX idx_oauth_tokens_service ON oauth_tokens(service);
CREATE INDEX idx_oauth_tokens_expires ON oauth_tokens(expires_at);

-- Coming soon/upcoming releases table
CREATE TABLE IF NOT EXISTS coming_soon_releases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tmdb_id INTEGER NOT NULL,
    media_type VARCHAR(20) NOT NULL CHECK (media_type IN ('movie', 'tv')),
    title VARCHAR(500) NOT NULL,
    release_date DATE NOT NULL,
    poster_path VARCHAR(255),
    backdrop_path VARCHAR(255),
    overview TEXT,
    source VARCHAR(50) NOT NULL,
    region VARCHAR(2) DEFAULT 'US',
    streaming_services JSONB, -- Array of service names where it will be available
    fetched_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tmdb_id, media_type, source, region)
);

-- Indexes for coming soon queries
CREATE INDEX idx_coming_soon_releases_date ON coming_soon_releases(release_date);
CREATE INDEX idx_coming_soon_releases_lookup ON coming_soon_releases(media_type, region, release_date);
CREATE INDEX idx_coming_soon_releases_expires ON coming_soon_releases(expires_at);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_streaming_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updated_at
CREATE TRIGGER update_streaming_cache_updated_at
    BEFORE UPDATE ON streaming_cache
    FOR EACH ROW
    EXECUTE FUNCTION update_streaming_updated_at();

CREATE TRIGGER update_streaming_id_mappings_updated_at
    BEFORE UPDATE ON streaming_id_mappings
    FOR EACH ROW
    EXECUTE FUNCTION update_streaming_updated_at();

CREATE TRIGGER update_oauth_tokens_updated_at
    BEFORE UPDATE ON oauth_tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_streaming_updated_at();

-- Comments for documentation
COMMENT ON TABLE streaming_cache IS 'Cache for external API responses with TTL-based expiration';
COMMENT ON TABLE streaming_id_mappings IS 'TMDB to Watchmode ID mapping from CSV sync';
COMMENT ON TABLE trending_entries IS 'Aggregated trending content from TMDB and Trakt';
COMMENT ON TABLE streaming_availability IS 'Streaming service availability with deep links';
COMMENT ON TABLE oauth_tokens IS 'OAuth tokens for external service authentication';
COMMENT ON TABLE coming_soon_releases IS 'Upcoming releases on streaming platforms';

COMMENT ON COLUMN streaming_cache.cache_key IS 'Format: service:endpoint:params_hash';
COMMENT ON COLUMN trending_entries.source IS 'Data source: tmdb, trakt, or aggregated (combined)';
COMMENT ON COLUMN trending_entries.score IS 'Normalized score 0-100 for aggregated ranking';
COMMENT ON COLUMN streaming_availability.deep_link IS 'Direct link to content on streaming platform';
COMMENT ON COLUMN oauth_tokens.expires_at IS 'Token expiry time - Trakt tokens expire after 24hrs as of March 2025';