-- Quality Engine Migration (Fixed for existing schema)
-- Handles existing quality_profiles table and adapts as needed

-- First, handle the existing quality_profiles table
-- Check if we need to modify it or if it's already in the right state
DO $$
BEGIN
    -- Check if quality_profiles exists with old schema
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'quality_profiles' AND column_name = 'cutoff_quality_id'
    ) THEN
        -- Add missing columns if they don't exist
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.columns 
            WHERE table_name = 'quality_profiles' AND column_name = 'preferred_qualities'
        ) THEN
            ALTER TABLE quality_profiles ADD COLUMN preferred_qualities JSONB NOT NULL DEFAULT '[]';
        END IF;
        
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.columns 
            WHERE table_name = 'quality_profiles' AND column_name = 'custom_format_scores'
        ) THEN
            ALTER TABLE quality_profiles ADD COLUMN custom_format_scores JSONB NOT NULL DEFAULT '{}';
        END IF;
        
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.columns 
            WHERE table_name = 'quality_profiles' AND column_name = 'min_size_mb'
        ) THEN
            ALTER TABLE quality_profiles ADD COLUMN min_size_mb INTEGER;
        END IF;
        
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.columns 
            WHERE table_name = 'quality_profiles' AND column_name = 'max_size_mb'
        ) THEN
            ALTER TABLE quality_profiles ADD COLUMN max_size_mb INTEGER;
        END IF;
    ELSE
        -- Create the table if it doesn't exist at all
        CREATE TABLE IF NOT EXISTS quality_profiles (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL UNIQUE,
            cutoff_quality_id INTEGER NOT NULL DEFAULT 8,
            preferred_qualities JSONB NOT NULL DEFAULT '[]',
            custom_format_scores JSONB NOT NULL DEFAULT '{}',
            min_size_mb INTEGER,
            max_size_mb INTEGER,
            upgrade_allowed BOOLEAN NOT NULL DEFAULT true,
            language TEXT NOT NULL DEFAULT 'english',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
    END IF;
END $$;

-- Create index on quality profile name for fast lookups
CREATE INDEX IF NOT EXISTS idx_quality_profiles_name ON quality_profiles(name);

-- Custom Formats table for advanced filtering rules
CREATE TABLE IF NOT EXISTS custom_formats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    specifications JSONB NOT NULL,
    score INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on custom format name
CREATE INDEX IF NOT EXISTS idx_custom_formats_name ON custom_formats(name);
CREATE INDEX IF NOT EXISTS idx_custom_formats_enabled ON custom_formats(enabled);

-- Quality Definitions table for standard quality levels
CREATE TABLE IF NOT EXISTS quality_definitions (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    weight INTEGER NOT NULL,
    min_size_mb INTEGER,
    max_size_mb INTEGER,
    preferred_size_mb INTEGER,
    resolution_width INTEGER,
    resolution_height INTEGER,
    modifier TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on quality definition weight for ordering
CREATE INDEX IF NOT EXISTS idx_quality_definitions_weight ON quality_definitions(weight);

-- Quality History table to track quality upgrades
CREATE TABLE IF NOT EXISTS quality_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movie_id UUID NOT NULL,
    old_quality_id INTEGER,
    new_quality_id INTEGER NOT NULL,
    old_custom_formats JSONB DEFAULT '[]',
    new_custom_formats JSONB DEFAULT '[]',
    old_file_path TEXT,
    new_file_path TEXT NOT NULL,
    reason TEXT NOT NULL,
    source_indexer TEXT,
    source_title TEXT,
    decision_score DECIMAL(10,2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indices for quality history
CREATE INDEX IF NOT EXISTS idx_quality_history_movie_id ON quality_history(movie_id);
CREATE INDEX IF NOT EXISTS idx_quality_history_created_at ON quality_history(created_at);
CREATE INDEX IF NOT EXISTS idx_quality_history_reason ON quality_history(reason);

-- Insert default quality definitions
INSERT INTO quality_definitions (name, title, weight, min_size_mb, max_size_mb, preferred_size_mb, resolution_width, resolution_height, modifier) VALUES
('Unknown', 'Unknown', 0, NULL, NULL, NULL, NULL, NULL, ''),
('SDTV', 'SDTV', 1, 150, 500, 300, 720, 576, ''),
('DVD', 'DVD', 2, 500, 2000, 1000, 720, 576, ''),
('WEBDL-480p', 'WEBDL-480p', 3, 200, 1000, 600, 854, 480, 'WEBDL'),
('HDTV-720p', 'HDTV-720p', 4, 750, 2500, 1500, 1280, 720, 'HDTV'),
('WEBDL-720p', 'WEBDL-720p', 5, 1000, 3000, 2000, 1280, 720, 'WEBDL'),
('Bluray-720p', 'Bluray-720p', 6, 2000, 7000, 4500, 1280, 720, 'Bluray'),
('HDTV-1080p', 'HDTV-1080p', 7, 1500, 4000, 2500, 1920, 1080, 'HDTV'),
('WEBDL-1080p', 'WEBDL-1080p', 8, 2000, 6000, 4000, 1920, 1080, 'WEBDL'),
('Bluray-1080p', 'Bluray-1080p', 9, 4000, 15000, 8000, 1920, 1080, 'Bluray'),
('Remux-1080p', 'Remux-1080p', 10, 15000, 35000, 20000, 1920, 1080, 'REMUX'),
('HDTV-2160p', 'HDTV-2160p', 11, 4000, 25000, 14000, 3840, 2160, 'HDTV'),
('WEBDL-2160p', 'WEBDL-2160p', 12, 6000, 25000, 15000, 3840, 2160, 'WEBDL'),
('Bluray-2160p', 'Bluray-2160p', 13, 15000, 50000, 25000, 3840, 2160, 'Bluray'),
('Remux-2160p', 'Remux-2160p', 14, 35000, 100000, 50000, 3840, 2160, 'REMUX')
ON CONFLICT (name) DO NOTHING;

-- Insert default custom formats
INSERT INTO custom_formats (name, specifications, score) VALUES
('Freeleech', '[{"type": "indexer_flag", "negate": false, "required": false, "value": "freeleech"}]', 25),
('Internal', '[{"type": "indexer_flag", "negate": false, "required": false, "value": "internal"}]', 15),
('Scene', '[{"type": "release_group", "negate": false, "required": false, "value": "scene"}]', -10),
('x265', '[{"type": "release_title", "negate": false, "required": false, "value": "x265|HEVC"}]', 5),
('Remux', '[{"type": "release_title", "negate": false, "required": false, "value": "remux"}]', 20),
('HDR', '[{"type": "release_title", "negate": false, "required": false, "value": "HDR|Dolby.*Vision"}]', 10),
('Atmos', '[{"type": "release_title", "negate": false, "required": false, "value": "Atmos|DTS:X"}]', 5),
('Multi', '[{"type": "release_title", "negate": false, "required": false, "value": "Multi|MULTI"}]', -5),
('High Seeders', '[{"type": "indexer_stats", "negate": false, "required": false, "value": "seeders>=20"}]', 5)
ON CONFLICT (name) DO NOTHING;