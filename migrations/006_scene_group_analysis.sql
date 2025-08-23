-- Migration: Scene Group Analysis Storage
-- Stores HDBits scene group analysis results for quality scoring

-- Scene Groups table (master list of all known scene groups)
CREATE TABLE IF NOT EXISTS scene_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) UNIQUE NOT NULL,
    group_type VARCHAR(50), -- 'internal', 'scene', 'p2p', 'unknown'
    reputation_score DECIMAL(5,2) DEFAULT 50.00, -- 0-100 scale
    confidence_level DECIMAL(5,2) DEFAULT 0.00, -- 0-100 based on sample size
    first_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_analyzed TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Scene Group Metrics table (detailed metrics per group)
CREATE TABLE IF NOT EXISTS scene_group_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_group_id UUID NOT NULL REFERENCES scene_groups(id) ON DELETE CASCADE,
    analysis_date DATE NOT NULL,
    release_count INTEGER DEFAULT 0,
    internal_release_count INTEGER DEFAULT 0,
    total_size_gb DECIMAL(10,2) DEFAULT 0.00,
    average_size_gb DECIMAL(10,2) DEFAULT 0.00,
    unique_movies INTEGER DEFAULT 0,
    freeleech_count INTEGER DEFAULT 0,
    freeleech_percentage DECIMAL(5,2) DEFAULT 0.00,
    average_seeders INTEGER DEFAULT 0,
    average_leechers INTEGER DEFAULT 0,
    codec_distribution JSONB DEFAULT '{}', -- {"x264": 50, "x265": 30, "AV1": 20}
    resolution_distribution JSONB DEFAULT '{}', -- {"1080p": 60, "2160p": 30, "720p": 10}
    source_distribution JSONB DEFAULT '{}', -- {"BluRay": 70, "WEB-DL": 20, "HDTV": 10}
    quality_metrics JSONB DEFAULT '{}', -- Additional quality indicators
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(scene_group_id, analysis_date)
);

-- Release Analysis table (individual releases analyzed)
CREATE TABLE IF NOT EXISTS release_analysis (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_group_id UUID REFERENCES scene_groups(id) ON DELETE SET NULL,
    release_name VARCHAR(500) NOT NULL,
    info_hash VARCHAR(64),
    imdb_id VARCHAR(20),
    tmdb_id INTEGER,
    title VARCHAR(500),
    year INTEGER,
    codec VARCHAR(20),
    resolution VARCHAR(20),
    source VARCHAR(50),
    size_gb DECIMAL(10,2),
    is_internal BOOLEAN DEFAULT false,
    is_freeleech BOOLEAN DEFAULT false,
    seeders INTEGER,
    leechers INTEGER,
    uploaded_at TIMESTAMP WITH TIME ZONE,
    analyzed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    quality_score DECIMAL(5,2), -- Individual release quality score
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Analysis Sessions table (tracks analysis runs)
CREATE TABLE IF NOT EXISTS analysis_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_type VARCHAR(50) NOT NULL, -- 'full', 'incremental', 'segmented'
    source VARCHAR(50) NOT NULL, -- 'hdbits', 'manual', 'automated'
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(50) DEFAULT 'running', -- 'running', 'completed', 'failed', 'partial'
    pages_processed INTEGER DEFAULT 0,
    releases_analyzed INTEGER DEFAULT 0,
    groups_discovered INTEGER DEFAULT 0,
    new_groups_added INTEGER DEFAULT 0,
    runtime_seconds INTEGER,
    segments_completed INTEGER DEFAULT 0,
    total_segments INTEGER DEFAULT 0,
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Scene Group Reputation History (tracks reputation changes over time)
CREATE TABLE IF NOT EXISTS scene_group_reputation_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scene_group_id UUID NOT NULL REFERENCES scene_groups(id) ON DELETE CASCADE,
    analysis_session_id UUID REFERENCES analysis_sessions(id) ON DELETE SET NULL,
    old_score DECIMAL(5,2),
    new_score DECIMAL(5,2),
    score_delta DECIMAL(5,2),
    reason VARCHAR(255),
    sample_size INTEGER, -- Number of releases used for calculation
    confidence DECIMAL(5,2), -- Statistical confidence in the score
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_scene_groups_name ON scene_groups(name);
CREATE INDEX idx_scene_groups_reputation ON scene_groups(reputation_score DESC);
CREATE INDEX idx_scene_groups_active ON scene_groups(is_active) WHERE is_active = true;
CREATE INDEX idx_scene_group_metrics_group ON scene_group_metrics(scene_group_id, analysis_date DESC);
CREATE INDEX idx_release_analysis_group ON release_analysis(scene_group_id);
CREATE INDEX idx_release_analysis_hash ON release_analysis(info_hash) WHERE info_hash IS NOT NULL;
CREATE INDEX idx_release_analysis_imdb ON release_analysis(imdb_id) WHERE imdb_id IS NOT NULL;
CREATE INDEX idx_release_analysis_tmdb ON release_analysis(tmdb_id) WHERE tmdb_id IS NOT NULL;
CREATE INDEX idx_release_analysis_date ON release_analysis(analyzed_at DESC);
CREATE INDEX idx_analysis_sessions_status ON analysis_sessions(status, started_at DESC);
CREATE INDEX idx_reputation_history_group ON scene_group_reputation_history(scene_group_id, created_at DESC);

-- Triggers for updated_at
CREATE TRIGGER update_scene_groups_updated_at BEFORE UPDATE ON scene_groups
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_analysis_sessions_updated_at BEFORE UPDATE ON analysis_sessions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to calculate reputation score based on metrics
CREATE OR REPLACE FUNCTION calculate_scene_group_reputation(
    p_scene_group_id UUID
)
RETURNS DECIMAL AS $$
DECLARE
    v_score DECIMAL(5,2);
    v_metrics RECORD;
    v_release_count INTEGER;
BEGIN
    -- Get latest metrics
    SELECT * INTO v_metrics
    FROM scene_group_metrics
    WHERE scene_group_id = p_scene_group_id
    ORDER BY analysis_date DESC
    LIMIT 1;
    
    IF NOT FOUND THEN
        RETURN 50.00; -- Default neutral score
    END IF;
    
    -- Get total release count
    SELECT COUNT(*) INTO v_release_count
    FROM release_analysis
    WHERE scene_group_id = p_scene_group_id;
    
    -- Base score starts at 50
    v_score := 50.00;
    
    -- Adjust based on release count (max +10)
    IF v_release_count > 100 THEN
        v_score := v_score + 10;
    ELSIF v_release_count > 50 THEN
        v_score := v_score + 7;
    ELSIF v_release_count > 20 THEN
        v_score := v_score + 5;
    ELSIF v_release_count > 10 THEN
        v_score := v_score + 3;
    END IF;
    
    -- Adjust based on internal percentage (max +15)
    IF v_metrics.internal_release_count > 0 THEN
        v_score := v_score + LEAST(15, (v_metrics.internal_release_count::DECIMAL / NULLIF(v_metrics.release_count, 0) * 15));
    END IF;
    
    -- Adjust based on freeleech percentage (max +10)
    IF v_metrics.freeleech_percentage > 0 THEN
        v_score := v_score + (v_metrics.freeleech_percentage / 10);
    END IF;
    
    -- Bonus for consistent activity (max +5)
    IF v_metrics.release_count > 5 THEN
        v_score := v_score + 5;
    END IF;
    
    -- Penalty for very low activity (max -10)
    IF v_release_count < 5 THEN
        v_score := v_score - 10;
    END IF;
    
    -- Ensure score is within bounds
    v_score := LEAST(100, GREATEST(0, v_score));
    
    RETURN v_score;
END;
$$ LANGUAGE plpgsql;

-- Function to update scene group from analysis
CREATE OR REPLACE FUNCTION update_scene_group_from_analysis(
    p_group_name VARCHAR(100),
    p_metrics JSONB,
    p_session_id UUID DEFAULT NULL
)
RETURNS UUID AS $$
DECLARE
    v_group_id UUID;
    v_old_score DECIMAL(5,2);
    v_new_score DECIMAL(5,2);
    v_sample_size INTEGER;
BEGIN
    -- Get or create scene group
    SELECT id, reputation_score INTO v_group_id, v_old_score
    FROM scene_groups
    WHERE name = p_group_name;
    
    IF NOT FOUND THEN
        INSERT INTO scene_groups (name, group_type, metadata)
        VALUES (p_group_name, 'unknown', p_metrics)
        RETURNING id INTO v_group_id;
        
        v_old_score := 50.00;
    END IF;
    
    -- Update last seen
    UPDATE scene_groups
    SET last_seen = NOW(),
        last_analyzed = NOW(),
        metadata = metadata || p_metrics
    WHERE id = v_group_id;
    
    -- Calculate new reputation score
    v_new_score := calculate_scene_group_reputation(v_group_id);
    
    -- Update reputation score
    UPDATE scene_groups
    SET reputation_score = v_new_score,
        confidence_level = LEAST(100, GREATEST(0, 
            (p_metrics->>'release_count')::INTEGER::DECIMAL / 10 * 10))
    WHERE id = v_group_id;
    
    -- Record reputation history
    IF p_session_id IS NOT NULL AND v_old_score != v_new_score THEN
        v_sample_size := COALESCE((p_metrics->>'release_count')::INTEGER, 0);
        
        INSERT INTO scene_group_reputation_history (
            scene_group_id, analysis_session_id, old_score, new_score, 
            score_delta, reason, sample_size, confidence
        ) VALUES (
            v_group_id, p_session_id, v_old_score, v_new_score,
            v_new_score - v_old_score, 'Analysis update', v_sample_size,
            LEAST(100, v_sample_size::DECIMAL / 10 * 10)
        );
    END IF;
    
    RETURN v_group_id;
END;
$$ LANGUAGE plpgsql;

-- View for current scene group standings
CREATE OR REPLACE VIEW scene_group_standings AS
SELECT 
    sg.name,
    sg.group_type,
    sg.reputation_score,
    sg.confidence_level,
    sgm.release_count,
    sgm.internal_release_count,
    sgm.freeleech_percentage,
    sgm.average_size_gb,
    sg.last_analyzed,
    sg.is_active
FROM scene_groups sg
LEFT JOIN LATERAL (
    SELECT *
    FROM scene_group_metrics
    WHERE scene_group_id = sg.id
    ORDER BY analysis_date DESC
    LIMIT 1
) sgm ON true
WHERE sg.is_active = true
ORDER BY sg.reputation_score DESC, sg.confidence_level DESC;

-- View for recent analysis sessions
CREATE OR REPLACE VIEW recent_analysis_sessions AS
SELECT 
    id,
    session_type,
    source,
    started_at,
    completed_at,
    status,
    pages_processed,
    releases_analyzed,
    groups_discovered,
    new_groups_added,
    runtime_seconds,
    CASE 
        WHEN runtime_seconds > 0 THEN 
            ROUND(releases_analyzed::DECIMAL / runtime_seconds * 60, 2)
        ELSE 0 
    END as releases_per_minute,
    segments_completed,
    total_segments
FROM analysis_sessions
ORDER BY started_at DESC
LIMIT 20;

-- Sample data for testing (commented out for production)
-- INSERT INTO scene_groups (name, group_type, reputation_score) VALUES
-- ('SPARKS', 'scene', 85.0),
-- ('RARBG', 'p2p', 75.0),
-- ('FGT', 'scene', 70.0),
-- ('EVO', 'internal', 90.0);

-- Comments for documentation
COMMENT ON TABLE scene_groups IS 'Master list of all known scene groups with reputation scores';
COMMENT ON TABLE scene_group_metrics IS 'Detailed metrics for scene groups collected during analysis';
COMMENT ON TABLE release_analysis IS 'Individual release data collected during analysis';
COMMENT ON TABLE analysis_sessions IS 'Tracks HDBits analysis runs and their results';
COMMENT ON TABLE scene_group_reputation_history IS 'Historical record of reputation score changes';
COMMENT ON COLUMN scene_groups.reputation_score IS 'Quality score from 0-100, higher is better';
COMMENT ON COLUMN scene_groups.confidence_level IS 'Statistical confidence in the reputation score based on sample size';
COMMENT ON FUNCTION calculate_scene_group_reputation IS 'Calculates reputation score based on release metrics';
COMMENT ON FUNCTION update_scene_group_from_analysis IS 'Updates scene group data from analysis results';