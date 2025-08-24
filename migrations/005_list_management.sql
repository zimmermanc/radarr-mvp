-- Migration: List Management System
-- Adds support for importing and syncing from IMDb, TMDb, Plex, and other list sources

-- Import Lists table (stores list configurations)
CREATE TABLE IF NOT EXISTS import_lists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    source_type VARCHAR(50) NOT NULL, -- 'imdb', 'tmdb', 'plex', 'trakt', 'letterboxd', 'custom'
    list_url TEXT, -- URL for public lists
    list_id VARCHAR(255), -- Platform-specific list ID
    enabled BOOLEAN DEFAULT true,
    sync_enabled BOOLEAN DEFAULT true,
    sync_interval INTEGER DEFAULT 360, -- Minutes between syncs (default 6 hours)
    add_monitored BOOLEAN DEFAULT true, -- Auto-monitor imported movies
    minimum_availability VARCHAR(50) DEFAULT 'announced', -- 'announced', 'inCinemas', 'released', 'preDB'
    quality_profile_id INTEGER REFERENCES quality_profiles(id),
    metadata_profile_id UUID, -- For future metadata profiles
    root_folder_path VARCHAR(500),
    tags TEXT[], -- Array of tags to apply to imported movies
    config_data JSONB, -- Platform-specific configuration
    last_sync_at TIMESTAMP WITH TIME ZONE,
    next_sync_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- List Items table (cached items from lists)
CREATE TABLE IF NOT EXISTS list_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    import_list_id UUID NOT NULL REFERENCES import_lists(id) ON DELETE CASCADE,
    tmdb_id INTEGER,
    imdb_id VARCHAR(20),
    title VARCHAR(500) NOT NULL,
    year INTEGER,
    overview TEXT,
    poster_path VARCHAR(500),
    backdrop_path VARCHAR(500),
    release_date DATE,
    runtime INTEGER, -- in minutes
    genres TEXT[],
    original_language VARCHAR(10),
    vote_average DECIMAL(3,1),
    vote_count INTEGER,
    popularity DECIMAL(10,2),
    status VARCHAR(50), -- 'pending', 'imported', 'excluded', 'existing'
    movie_id UUID REFERENCES movies(id), -- Link to imported movie if exists
    excluded_reason VARCHAR(255), -- Why item was excluded from import
    metadata JSONB, -- Additional platform-specific metadata
    discovered_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    imported_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- List Sync History table (tracks sync operations)
CREATE TABLE IF NOT EXISTS list_sync_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    import_list_id UUID NOT NULL REFERENCES import_lists(id) ON DELETE CASCADE,
    sync_status VARCHAR(50) NOT NULL, -- 'started', 'success', 'partial', 'failed'
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    duration_ms INTEGER,
    items_found INTEGER DEFAULT 0,
    items_added INTEGER DEFAULT 0,
    items_updated INTEGER DEFAULT 0,
    items_removed INTEGER DEFAULT 0,
    items_excluded INTEGER DEFAULT 0,
    error_message TEXT,
    error_details JSONB,
    sync_metadata JSONB -- Additional sync information
);

-- List Sync Jobs table (job scheduling and tracking)
CREATE TABLE IF NOT EXISTS list_sync_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    import_list_id UUID REFERENCES import_lists(id) ON DELETE CASCADE,
    job_type VARCHAR(50) NOT NULL, -- 'manual', 'scheduled', 'retry'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed', 'cancelled'
    priority INTEGER DEFAULT 5, -- 1-10, higher = more priority
    scheduled_at TIMESTAMP WITH TIME ZONE NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- List Exclusions table (movies to never import from lists)
CREATE TABLE IF NOT EXISTS list_exclusions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tmdb_id INTEGER,
    imdb_id VARCHAR(20),
    title VARCHAR(500) NOT NULL,
    year INTEGER,
    reason VARCHAR(255),
    excluded_by VARCHAR(100), -- Username or system
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(tmdb_id),
    UNIQUE(imdb_id)
);

-- Movie Provenance table (tracks which list added which movie)
CREATE TABLE IF NOT EXISTS movie_provenance (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movie_id UUID NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    import_list_id UUID REFERENCES import_lists(id) ON DELETE SET NULL,
    source_type VARCHAR(50) NOT NULL, -- 'list', 'manual', 'rss', 'recommendation'
    source_name VARCHAR(255),
    added_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB -- Additional source information
);

-- Indexes for performance
CREATE INDEX idx_import_lists_enabled ON import_lists(enabled) WHERE enabled = true;
CREATE INDEX idx_import_lists_next_sync ON import_lists(next_sync_at) WHERE enabled = true AND sync_enabled = true;
CREATE INDEX idx_list_items_import_list ON list_items(import_list_id);
CREATE INDEX idx_list_items_tmdb ON list_items(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_list_items_imdb ON list_items(imdb_id) WHERE imdb_id IS NOT NULL;
CREATE INDEX idx_list_items_status ON list_items(status);
CREATE INDEX idx_list_items_movie ON list_items(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_list_sync_history_list ON list_sync_history(import_list_id, started_at DESC);
CREATE INDEX idx_list_sync_jobs_status ON list_sync_jobs(status, scheduled_at) WHERE status IN ('pending', 'running');
CREATE INDEX idx_list_sync_jobs_list ON list_sync_jobs(import_list_id, created_at DESC);
CREATE INDEX idx_list_exclusions_tmdb ON list_exclusions(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_list_exclusions_imdb ON list_exclusions(imdb_id) WHERE imdb_id IS NOT NULL;
CREATE INDEX idx_movie_provenance_movie ON movie_provenance(movie_id);
CREATE INDEX idx_movie_provenance_list ON movie_provenance(import_list_id) WHERE import_list_id IS NOT NULL;

-- Triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_import_lists_updated_at BEFORE UPDATE ON import_lists
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_list_items_updated_at BEFORE UPDATE ON list_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_list_sync_jobs_updated_at BEFORE UPDATE ON list_sync_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Helper function to check if a movie should be imported
CREATE OR REPLACE FUNCTION should_import_movie(
    p_tmdb_id INTEGER,
    p_imdb_id VARCHAR(20)
)
RETURNS BOOLEAN AS $$
BEGIN
    -- Check if movie is in exclusion list
    IF EXISTS (
        SELECT 1 FROM list_exclusions 
        WHERE (tmdb_id = p_tmdb_id AND p_tmdb_id IS NOT NULL)
           OR (imdb_id = p_imdb_id AND p_imdb_id IS NOT NULL)
    ) THEN
        RETURN FALSE;
    END IF;
    
    -- Check if movie already exists in library
    IF EXISTS (
        SELECT 1 FROM movies 
        WHERE (tmdb_id = p_tmdb_id AND p_tmdb_id IS NOT NULL)
           OR (imdb_id = p_imdb_id AND p_imdb_id IS NOT NULL)
    ) THEN
        RETURN FALSE;
    END IF;
    
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- Function to schedule next sync for a list
CREATE OR REPLACE FUNCTION schedule_next_list_sync(
    p_import_list_id UUID
)
RETURNS TIMESTAMP WITH TIME ZONE AS $$
DECLARE
    v_interval INTEGER;
    v_next_sync TIMESTAMP WITH TIME ZONE;
BEGIN
    SELECT sync_interval INTO v_interval
    FROM import_lists
    WHERE id = p_import_list_id AND enabled = true AND sync_enabled = true;
    
    IF v_interval IS NULL THEN
        RETURN NULL;
    END IF;
    
    v_next_sync := NOW() + (v_interval || ' minutes')::INTERVAL;
    
    UPDATE import_lists
    SET next_sync_at = v_next_sync
    WHERE id = p_import_list_id;
    
    RETURN v_next_sync;
END;
$$ LANGUAGE plpgsql;

-- Sample data for testing (commented out for production)
-- INSERT INTO import_lists (name, source_type, list_url, sync_interval) VALUES
-- ('IMDb Top 250', 'imdb', 'https://www.imdb.com/chart/top', 1440),
-- ('TMDb Popular', 'tmdb', 'popular', 360),
-- ('TMDb Upcoming', 'tmdb', 'upcoming', 720),
-- ('Trakt Trending', 'trakt', 'trending', 180);

-- Comments for documentation
COMMENT ON TABLE import_lists IS 'Stores configuration for movie import lists from various sources';
COMMENT ON TABLE list_items IS 'Cached items from import lists awaiting processing';
COMMENT ON TABLE list_sync_history IS 'Historical record of list synchronization operations';
COMMENT ON TABLE list_sync_jobs IS 'Queue for list sync jobs to be processed';
COMMENT ON TABLE list_exclusions IS 'Movies that should never be imported from lists';
COMMENT ON TABLE movie_provenance IS 'Tracks the source of how movies were added to the library';
COMMENT ON COLUMN import_lists.source_type IS 'Platform identifier: imdb, tmdb, plex, trakt, letterboxd, custom';
COMMENT ON COLUMN import_lists.minimum_availability IS 'Minimum availability status required for import: announced, inCinemas, released, preDB';
COMMENT ON COLUMN list_items.status IS 'Import status: pending, imported, excluded, existing';
COMMENT ON COLUMN list_sync_history.sync_status IS 'Sync operation result: started, success, partial, failed';
COMMENT ON COLUMN list_sync_jobs.job_type IS 'Type of sync job: manual, scheduled, retry';