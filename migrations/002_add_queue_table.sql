-- Add queue management table for download tracking
-- This migration adds comprehensive queue functionality with qBittorrent integration

-- Queue items table for download queue management
CREATE TABLE queue (
    -- Core identification
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movie_id UUID NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    release_id UUID NOT NULL, -- References release from indexer search
    
    -- Download information
    title TEXT NOT NULL,
    download_url TEXT NOT NULL,
    magnet_url TEXT,
    size_bytes BIGINT,
    
    -- Status and priority tracking
    status TEXT NOT NULL DEFAULT 'queued',
    -- queued, downloading, completed, failed, cancelled, paused, stalled, seeding
    priority TEXT NOT NULL DEFAULT 'normal',
    -- low, normal, high, very_high
    progress NUMERIC(5,4) NOT NULL DEFAULT 0.0, -- 0.0 to 1.0
    
    -- Download client integration
    download_client_id TEXT, -- ID from download client (e.g., torrent hash)
    download_path TEXT,
    category TEXT,
    
    -- Progress tracking metrics
    downloaded_bytes BIGINT,
    upload_bytes BIGINT,
    download_speed BIGINT, -- bytes per second
    upload_speed BIGINT,   -- bytes per second
    eta_seconds BIGINT,    -- estimated time remaining
    seeders INTEGER,
    leechers INTEGER,
    
    -- Error handling and retry logic
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    
    -- Timestamps for tracking
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

-- Indexes for queue performance
CREATE INDEX idx_queue_movie_id ON queue (movie_id);
CREATE INDEX idx_queue_status ON queue (status);
CREATE INDEX idx_queue_priority ON queue (priority);
CREATE INDEX idx_queue_created_at ON queue (created_at);
CREATE INDEX idx_queue_download_client_id ON queue (download_client_id) WHERE download_client_id IS NOT NULL;

-- Composite indexes for common queries
CREATE INDEX idx_queue_status_priority ON queue (status, priority);
CREATE INDEX idx_queue_movie_status ON queue (movie_id, status);

-- Partial indexes for active items only
CREATE INDEX idx_queue_active ON queue (created_at) 
    WHERE status IN ('queued', 'downloading', 'seeding');

-- Add trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_queue_updated_at
    BEFORE UPDATE ON queue
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add check constraints for data validation
ALTER TABLE queue ADD CONSTRAINT chk_queue_status 
    CHECK (status IN ('queued', 'downloading', 'completed', 'failed', 'cancelled', 'paused', 'stalled', 'seeding'));

ALTER TABLE queue ADD CONSTRAINT chk_queue_priority 
    CHECK (priority IN ('low', 'normal', 'high', 'very_high'));

ALTER TABLE queue ADD CONSTRAINT chk_queue_progress 
    CHECK (progress >= 0.0 AND progress <= 1.0);

ALTER TABLE queue ADD CONSTRAINT chk_queue_retry_count 
    CHECK (retry_count >= 0 AND retry_count <= max_retries);

-- Add comments for documentation
COMMENT ON TABLE queue IS 'Download queue management table for tracking movie downloads through various clients';
COMMENT ON COLUMN queue.release_id IS 'References the release that was selected for download (from indexer search results)';
COMMENT ON COLUMN queue.download_client_id IS 'Client-specific identifier (e.g., torrent hash for qBittorrent)';
COMMENT ON COLUMN queue.progress IS 'Download progress from 0.0 to 1.0 (0% to 100%)';
COMMENT ON COLUMN queue.eta_seconds IS 'Estimated time remaining in seconds (-1 if unknown)';
COMMENT ON COLUMN queue.retry_count IS 'Number of retry attempts made for failed downloads';