-- Migration: Blocklist System for Failed Releases
-- Adds comprehensive blocklist functionality to track and manage failed releases
-- preventing unnecessary retry attempts and improving system efficiency.

-- Blocklist table stores failed releases with TTL-based retry logic
CREATE TABLE IF NOT EXISTS blocklist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    release_id VARCHAR(500) NOT NULL, -- Release ID or GUID from indexer
    indexer VARCHAR(255) NOT NULL, -- Name of the indexer that provided this release
    reason VARCHAR(50) NOT NULL, -- Failure reason type (enum as string)
    reason_detail VARCHAR(255), -- Specific failure detail for ImportFailed types
    blocked_until TIMESTAMP WITH TIME ZONE NOT NULL, -- When this entry expires and can be retried
    retry_count INTEGER DEFAULT 0, -- Number of retry attempts made
    movie_id UUID REFERENCES movies(id) ON DELETE CASCADE, -- Optional movie this release was for
    release_title VARCHAR(1000) NOT NULL, -- Title of the blocked release for display
    metadata JSONB, -- Additional failure metadata (original error, context)
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Ensure unique blocklist entries per release/indexer combination
    UNIQUE(release_id, indexer)
);

-- Indexes for efficient blocklist operations
CREATE INDEX idx_blocklist_release_indexer ON blocklist(release_id, indexer);
CREATE INDEX idx_blocklist_indexer ON blocklist(indexer);
CREATE INDEX idx_blocklist_reason ON blocklist(reason);
CREATE INDEX idx_blocklist_movie ON blocklist(movie_id) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_blocklist_blocked_until ON blocklist(blocked_until);
-- Removed immutable function calls from index predicates
-- CREATE INDEX idx_blocklist_expired ON blocklist(blocked_until) WHERE blocked_until <= NOW();
-- CREATE INDEX idx_blocklist_active ON blocklist(blocked_until) WHERE blocked_until > NOW();
CREATE INDEX idx_blocklist_created_at ON blocklist(created_at DESC);
CREATE INDEX idx_blocklist_retry_count ON blocklist(retry_count);

-- Composite indexes for common query patterns
CREATE INDEX idx_blocklist_indexer_reason ON blocklist(indexer, reason);
CREATE INDEX idx_blocklist_movie_reason ON blocklist(movie_id, reason) WHERE movie_id IS NOT NULL;
CREATE INDEX idx_blocklist_expiry_retry ON blocklist(blocked_until, retry_count);

-- Partial indexes for specific failure types (performance optimization)
CREATE INDEX idx_blocklist_permanent ON blocklist(created_at DESC) 
WHERE reason IN ('ManuallyRejected', 'QualityRejected', 'SizeRejected', 'ExclusionMatched');

CREATE INDEX idx_blocklist_retryable ON blocklist(blocked_until) 
WHERE reason NOT IN ('ManuallyRejected', 'QualityRejected', 'SizeRejected', 'ExclusionMatched') 
AND retry_count < 10; -- Reasonable max retry limit

-- Function to check if a release is currently blocked
CREATE OR REPLACE FUNCTION is_release_blocked(
    p_release_id VARCHAR(500),
    p_indexer VARCHAR(255)
)
RETURNS BOOLEAN AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1 FROM blocklist 
        WHERE release_id = p_release_id 
        AND indexer = p_indexer 
        AND blocked_until > NOW()
    );
END;
$$ LANGUAGE plpgsql;

-- Function to get the appropriate retry delay for a failure reason
CREATE OR REPLACE FUNCTION get_retry_delay(
    p_reason VARCHAR(50),
    p_reason_detail VARCHAR(255),
    p_retry_count INTEGER DEFAULT 0
)
RETURNS INTERVAL AS $$
DECLARE
    base_delay INTERVAL;
    backoff_multiplier FLOAT;
BEGIN
    -- Set base delay based on failure reason
    CASE p_reason
        WHEN 'AuthenticationFailed' THEN base_delay := '6 hours';
        WHEN 'ConnectionTimeout' THEN base_delay := '2 hours';
        WHEN 'NetworkError' THEN base_delay := '1 hour';
        WHEN 'ServerError' THEN base_delay := '30 minutes';
        WHEN 'RateLimited' THEN base_delay := '1 hour';
        WHEN 'ParseError' THEN base_delay := '4 hours';
        WHEN 'DiskFull' THEN base_delay := '30 minutes';
        WHEN 'PermissionDenied' THEN base_delay := '2 hours';
        WHEN 'DownloadClientError' THEN base_delay := '1 hour';
        WHEN 'DownloadStalled' THEN base_delay := '2 hours';
        WHEN 'HashMismatch' THEN base_delay := '4 hours';
        WHEN 'CorruptedDownload' THEN base_delay := '6 hours';
        WHEN 'QualityRejected' THEN base_delay := '7 days';
        WHEN 'SizeRejected' THEN base_delay := '3 days';
        WHEN 'ExclusionMatched' THEN base_delay := '30 days';
        WHEN 'ManuallyRejected' THEN base_delay := '30 days';
        WHEN 'ReleasePurged' THEN base_delay := '7 days';
        WHEN 'ImportFailed' THEN
            CASE p_reason_detail
                WHEN 'UnsupportedFormat' THEN base_delay := '1 day';
                WHEN 'FileAlreadyExists' THEN base_delay := '1 day';
                WHEN 'QualityAnalysisFailed' THEN base_delay := '6 hours';
                WHEN 'MediaInfoFailed' THEN base_delay := '6 hours';
                WHEN 'FileMoveError' THEN base_delay := '30 minutes';
                WHEN 'DirectoryCreationFailed' THEN base_delay := '1 hour';
                WHEN 'FilenameParseFailed' THEN base_delay := '12 hours';
                ELSE base_delay := '2 hours';
            END CASE;
        ELSE base_delay := '2 hours';
    END CASE;
    
    -- Apply exponential backoff for retryable failures
    IF p_reason IN ('ConnectionTimeout', 'NetworkError', 'ServerError', 'DownloadStalled', 'DownloadClientError', 'ParseError') 
       AND p_retry_count > 0 THEN
        backoff_multiplier := power(2, least(p_retry_count, 4)); -- Cap at 16x
        base_delay := base_delay * backoff_multiplier;
    END IF;
    
    RETURN base_delay;
END;
$$ LANGUAGE plpgsql;

-- Function to get maximum retry attempts for a failure reason
CREATE OR REPLACE FUNCTION get_max_retries(
    p_reason VARCHAR(50),
    p_reason_detail VARCHAR(255) DEFAULT NULL
)
RETURNS INTEGER AS $$
BEGIN
    CASE p_reason
        -- Permanent failures - no retries
        WHEN 'ManuallyRejected', 'QualityRejected', 'SizeRejected', 'ExclusionMatched' THEN RETURN 0;
        WHEN 'ImportFailed' THEN
            CASE p_reason_detail
                WHEN 'UnsupportedFormat', 'FileAlreadyExists' THEN RETURN 0;
                ELSE RETURN 3;
            END CASE;
        -- Authentication and access issues - few retries
        WHEN 'AuthenticationFailed', 'PermissionDenied' THEN RETURN 2;
        -- Rate limiting - limited retries
        WHEN 'RateLimited' THEN RETURN 3;
        -- Network and connectivity - moderate retries  
        WHEN 'ConnectionTimeout' THEN RETURN 5;
        WHEN 'NetworkError' THEN RETURN 4;
        WHEN 'ServerError' THEN RETURN 3;
        -- Download issues - more retries as they may be transient
        WHEN 'DownloadStalled', 'DownloadClientError' THEN RETURN 4;
        WHEN 'CorruptedDownload' THEN RETURN 3;
        WHEN 'HashMismatch' THEN RETURN 2;
        -- Resource constraints - more retries as they resolve over time
        WHEN 'DiskFull' THEN RETURN 10;
        -- Parse issues - moderate retries
        WHEN 'ParseError' THEN RETURN 3;
        -- Purged releases - single retry after delay
        WHEN 'ReleasePurged' THEN RETURN 1;
        ELSE RETURN 3;
    END CASE;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up old expired entries
CREATE OR REPLACE FUNCTION cleanup_blocklist_entries(
    p_older_than_days INTEGER DEFAULT 30
)
RETURNS INTEGER AS $$
DECLARE
    rows_deleted INTEGER;
BEGIN
    DELETE FROM blocklist 
    WHERE created_at < NOW() - (p_older_than_days || ' days')::INTERVAL
    AND blocked_until < NOW()
    AND (retry_count >= get_max_retries(reason, reason_detail) OR reason IN ('ManuallyRejected', 'QualityRejected', 'SizeRejected', 'ExclusionMatched'));
    
    GET DIAGNOSTICS rows_deleted = ROW_COUNT;
    RETURN rows_deleted;
END;
$$ LANGUAGE plpgsql;

-- Trigger to automatically set updated_at timestamp
CREATE OR REPLACE FUNCTION update_blocklist_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_blocklist_updated_at
    BEFORE UPDATE ON blocklist
    FOR EACH ROW
    EXECUTE FUNCTION update_blocklist_updated_at();

-- View for blocklist statistics (for monitoring and dashboards)
CREATE OR REPLACE VIEW blocklist_stats AS
SELECT 
    COUNT(*) FILTER (WHERE blocked_until > NOW()) as active_entries,
    COUNT(*) FILTER (WHERE blocked_until <= NOW()) as expired_entries,
    COUNT(*) FILTER (WHERE reason IN ('ManuallyRejected', 'QualityRejected', 'SizeRejected', 'ExclusionMatched')) as permanent_blocks,
    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as recent_additions,
    -- Most common failure reason
    (SELECT reason FROM blocklist GROUP BY reason ORDER BY COUNT(*) DESC LIMIT 1) as top_failure_reason,
    -- Count for top failure reason
    (SELECT COUNT(*) FROM blocklist WHERE reason = (SELECT reason FROM blocklist GROUP BY reason ORDER BY COUNT(*) DESC LIMIT 1)) as top_failure_count,
    -- Most problematic indexer
    (SELECT indexer FROM blocklist GROUP BY indexer ORDER BY COUNT(*) DESC LIMIT 1) as top_failing_indexer,
    -- Count for top failing indexer
    (SELECT COUNT(*) FROM blocklist WHERE indexer = (SELECT indexer FROM blocklist GROUP BY indexer ORDER BY COUNT(*) DESC LIMIT 1)) as top_indexer_failure_count;

-- View for failure reason analysis
CREATE OR REPLACE VIEW blocklist_failure_analysis AS
SELECT 
    reason,
    reason_detail,
    COUNT(*) FILTER (WHERE blocked_until > NOW()) as active_count,
    COUNT(*) FILTER (WHERE blocked_until <= NOW()) as expired_count,
    COUNT(*) as total_count,
    AVG(retry_count) as avg_retries,
    MAX(retry_count) as max_retries,
    AVG(EXTRACT(EPOCH FROM (blocked_until - created_at))/3600) as avg_block_duration_hours
FROM blocklist 
GROUP BY reason, reason_detail
ORDER BY total_count DESC;

-- Comments for documentation
COMMENT ON TABLE blocklist IS 'Stores failed releases to prevent unnecessary retry attempts';
COMMENT ON COLUMN blocklist.release_id IS 'Release ID or GUID from the indexer';
COMMENT ON COLUMN blocklist.indexer IS 'Name of the indexer that provided this release';
COMMENT ON COLUMN blocklist.reason IS 'Primary failure reason category';
COMMENT ON COLUMN blocklist.reason_detail IS 'Specific failure detail for complex failures like ImportFailed';
COMMENT ON COLUMN blocklist.blocked_until IS 'Timestamp when this entry expires and release can be retried';
COMMENT ON COLUMN blocklist.retry_count IS 'Number of retry attempts made for this release';
COMMENT ON COLUMN blocklist.metadata IS 'Additional failure context including original error details';

COMMENT ON FUNCTION is_release_blocked(VARCHAR, VARCHAR) IS 'Check if a release is currently blocked';
COMMENT ON FUNCTION get_retry_delay(VARCHAR, VARCHAR, INTEGER) IS 'Calculate appropriate retry delay with exponential backoff';
COMMENT ON FUNCTION get_max_retries(VARCHAR, VARCHAR) IS 'Get maximum retry attempts for a failure type';
COMMENT ON FUNCTION cleanup_blocklist_entries(INTEGER) IS 'Clean up old expired entries to prevent table bloat';

COMMENT ON VIEW blocklist_stats IS 'Real-time statistics about blocklist entries for monitoring';
COMMENT ON VIEW blocklist_failure_analysis IS 'Detailed analysis of failure patterns for troubleshooting';

-- Sample cleanup job (run daily via cron or background job)
-- SELECT cleanup_blocklist_entries(30); -- Clean entries older than 30 days

-- Validation constraints
ALTER TABLE blocklist ADD CONSTRAINT chk_retry_count_non_negative 
    CHECK (retry_count >= 0);

ALTER TABLE blocklist ADD CONSTRAINT chk_blocked_until_future_or_past 
    CHECK (blocked_until >= created_at);

ALTER TABLE blocklist ADD CONSTRAINT chk_release_id_not_empty 
    CHECK (length(trim(release_id)) > 0);

ALTER TABLE blocklist ADD CONSTRAINT chk_indexer_not_empty 
    CHECK (length(trim(indexer)) > 0);

ALTER TABLE blocklist ADD CONSTRAINT chk_release_title_not_empty 
    CHECK (length(trim(release_title)) > 0);

-- Enum constraint for reason field (validate against known failure types)
ALTER TABLE blocklist ADD CONSTRAINT chk_reason_valid 
    CHECK (reason IN (
        'ConnectionTimeout',
        'AuthenticationFailed', 
        'RateLimited',
        'ParseError',
        'DownloadStalled',
        'HashMismatch',
        'ImportFailed',
        'DiskFull',
        'PermissionDenied',
        'ManuallyRejected',
        'QualityRejected',
        'SizeRejected',
        'ReleasePurged',
        'NetworkError',
        'ServerError',
        'CorruptedDownload',
        'DownloadClientError',
        'ExclusionMatched'
    ));

-- Enum constraint for import failure details
ALTER TABLE blocklist ADD CONSTRAINT chk_import_failure_detail_valid
    CHECK (
        (reason != 'ImportFailed') OR 
        (reason = 'ImportFailed' AND reason_detail IN (
            'FileMoveError',
            'FileAlreadyExists',
            'DirectoryCreationFailed',
            'UnsupportedFormat',
            'QualityAnalysisFailed',
            'FilenameParseFailed',
            'MediaInfoFailed'
        ))
    );

-- Performance note: Consider partitioning by created_at if table grows very large
-- CREATE TABLE blocklist_y2025m01 PARTITION OF blocklist FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');