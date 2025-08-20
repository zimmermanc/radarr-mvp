-- =============================================================================
-- PostgreSQL Database Initialization Script for Radarr MVP
-- =============================================================================
-- This script runs when the PostgreSQL container starts for the first time
-- It sets up the database with optimal settings for Radarr MVP

-- Create database if it doesn't exist (handled by POSTGRES_DB env var)
-- CREATE DATABASE IF NOT EXISTS is not available in PostgreSQL, handled by container

-- Connect to the radarr database
\c radarr;

-- =============================================================================
-- DATABASE CONFIGURATION
-- =============================================================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- For text similarity searches
CREATE EXTENSION IF NOT EXISTS "btree_gin";   -- For GIN indexes on multiple columns

-- =============================================================================
-- PERFORMANCE OPTIMIZATIONS
-- =============================================================================

-- Set some performance-related settings for the session
-- Note: These are session-level settings. For persistent settings,
-- they should be set in postgresql.conf or via ALTER SYSTEM

-- Optimize for OLTP workloads (many small transactions)
SET random_page_cost = 1.1;  -- Assuming SSD storage
SET effective_io_concurrency = 200;  -- For SSD
SET seq_page_cost = 1.0;

-- =============================================================================
-- SCHEMA VALIDATION
-- =============================================================================

-- Create a simple validation table that can be used by the application
-- to verify database connectivity and schema state
CREATE TABLE IF NOT EXISTS schema_info (
    version VARCHAR(50) PRIMARY KEY,
    description TEXT,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Insert initial schema version
INSERT INTO schema_info (version, description) 
VALUES ('1.0.0', 'Initial Radarr MVP database setup')
ON CONFLICT (version) DO NOTHING;

-- =============================================================================
-- MONITORING TABLES
-- =============================================================================

-- Create a table for storing application health check results
CREATE TABLE IF NOT EXISTS health_checks (
    id SERIAL PRIMARY KEY,
    service_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('healthy', 'unhealthy', 'degraded')),
    details JSONB,
    checked_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create index for efficient health check queries
CREATE INDEX IF NOT EXISTS idx_health_checks_service_time 
ON health_checks(service_name, checked_at DESC);

-- =============================================================================
-- USER AND PERMISSIONS SETUP
-- =============================================================================

-- The main radarr user is already created by the container
-- Here we can set up additional permissions or roles if needed

-- Grant usage on all sequences (for SERIAL columns)
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO radarr;

-- Grant permissions on future sequences
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO radarr;

-- =============================================================================
-- LOGGING AND AUDITING SETUP
-- =============================================================================

-- Create a function to log important database operations
CREATE OR REPLACE FUNCTION log_database_operation()
RETURNS TRIGGER AS $$
BEGIN
    -- Log to PostgreSQL's log (visible in container logs)
    RAISE LOG 'Table % operation % by user %', TG_TABLE_NAME, TG_OP, current_user;
    
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- DATABASE MAINTENANCE
-- =============================================================================

-- Create a function to clean up old health check records
CREATE OR REPLACE FUNCTION cleanup_old_health_checks(retention_days INTEGER DEFAULT 7)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM health_checks 
    WHERE checked_at < CURRENT_TIMESTAMP - (retention_days || ' days')::INTERVAL;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    RAISE LOG 'Cleaned up % old health check records', deleted_count;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- INITIAL DATA SETUP
-- =============================================================================

-- Insert an initial health check record to verify the table works
INSERT INTO health_checks (service_name, status, details) 
VALUES (
    'database_init', 
    'healthy', 
    '{"message": "Database initialized successfully", "timestamp": "' || CURRENT_TIMESTAMP::TEXT || '"}'
);

-- =============================================================================
-- VACUUM AND ANALYZE
-- =============================================================================

-- Run VACUUM and ANALYZE on system tables to ensure good performance
VACUUM ANALYZE;

-- =============================================================================
-- COMPLETION LOG
-- =============================================================================

-- Log successful completion
DO $$
BEGIN
    RAISE LOG 'Radarr MVP database initialization completed successfully';
    RAISE LOG 'Database: %, User: %, Extensions loaded: uuid-ossp, pg_trgm, btree_gin', 
              current_database(), current_user;
END
$$;