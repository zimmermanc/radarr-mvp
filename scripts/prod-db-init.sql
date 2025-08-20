-- =============================================================================
-- Production Database Initialization Script for Radarr MVP
-- =============================================================================
-- This script runs in production with security and performance optimizations

-- Connect to the production database
\c radarr;

-- =============================================================================
-- PRODUCTION EXTENSIONS
-- =============================================================================

-- Enable required extensions for production
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- For text similarity searches
CREATE EXTENSION IF NOT EXISTS "btree_gin";   -- For GIN indexes on multiple columns

-- =============================================================================
-- PRODUCTION SCHEMA INFO
-- =============================================================================

CREATE TABLE IF NOT EXISTS schema_info (
    version VARCHAR(50) PRIMARY KEY,
    description TEXT,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    environment VARCHAR(20) DEFAULT 'production'
);

-- Insert production schema version
INSERT INTO schema_info (version, description, environment) 
VALUES ('1.0.0-prod', 'Production Radarr MVP database setup', 'production')
ON CONFLICT (version) DO NOTHING;

-- =============================================================================
-- PRODUCTION HEALTH CHECKS
-- =============================================================================

CREATE TABLE IF NOT EXISTS health_checks (
    id SERIAL PRIMARY KEY,
    service_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('healthy', 'unhealthy', 'degraded')),
    details JSONB,
    checked_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    environment VARCHAR(20) DEFAULT 'production'
);

-- Create optimized indexes for production workload
CREATE INDEX IF NOT EXISTS idx_health_checks_service_time 
ON health_checks(service_name, checked_at DESC);

CREATE INDEX IF NOT EXISTS idx_health_checks_status_time
ON health_checks(status, checked_at DESC) 
WHERE environment = 'production';

-- =============================================================================
-- PRODUCTION MONITORING TABLES
-- =============================================================================

-- Application metrics table for production monitoring
CREATE TABLE IF NOT EXISTS app_metrics (
    id SERIAL PRIMARY KEY,
    metric_name VARCHAR(100) NOT NULL,
    metric_value NUMERIC NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- 'counter', 'gauge', 'histogram'
    labels JSONB DEFAULT '{}',
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient metrics queries
CREATE INDEX IF NOT EXISTS idx_app_metrics_name_time
ON app_metrics(metric_name, recorded_at DESC);

-- Partial index for recent metrics (last 24 hours)
CREATE INDEX IF NOT EXISTS idx_app_metrics_recent
ON app_metrics(metric_name, recorded_at DESC)
WHERE recorded_at > (CURRENT_TIMESTAMP - INTERVAL '24 hours');

-- =============================================================================
-- PRODUCTION ERROR LOGGING
-- =============================================================================

-- Error logs table for production error tracking
CREATE TABLE IF NOT EXISTS error_logs (
    id SERIAL PRIMARY KEY,
    error_level VARCHAR(20) NOT NULL, -- 'ERROR', 'WARN', 'FATAL'
    error_message TEXT NOT NULL,
    error_context JSONB DEFAULT '{}',
    service_name VARCHAR(100),
    request_id UUID,
    user_id VARCHAR(100),
    occurred_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for production error analysis
CREATE INDEX IF NOT EXISTS idx_error_logs_level_time
ON error_logs(error_level, occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_error_logs_service_time
ON error_logs(service_name, occurred_at DESC);

-- =============================================================================
-- PRODUCTION AUDIT TRAIL
-- =============================================================================

-- Audit trail for important operations in production
CREATE TABLE IF NOT EXISTS audit_trail (
    id SERIAL PRIMARY KEY,
    table_name VARCHAR(100) NOT NULL,
    operation VARCHAR(10) NOT NULL, -- 'INSERT', 'UPDATE', 'DELETE'
    record_id VARCHAR(100),
    old_values JSONB,
    new_values JSONB,
    changed_by VARCHAR(100),
    changed_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    ip_address INET,
    user_agent TEXT
);

-- Index for audit queries
CREATE INDEX IF NOT EXISTS idx_audit_trail_table_time
ON audit_trail(table_name, changed_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_trail_user_time
ON audit_trail(changed_by, changed_at DESC);

-- =============================================================================
-- PRODUCTION PERFORMANCE FUNCTIONS
-- =============================================================================

-- Function to clean up old metrics (retention policy)
CREATE OR REPLACE FUNCTION cleanup_old_metrics(retention_hours INTEGER DEFAULT 168) -- 7 days default
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM app_metrics 
    WHERE recorded_at < CURRENT_TIMESTAMP - (retention_hours || ' hours')::INTERVAL;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    -- Log cleanup operation
    INSERT INTO app_metrics (metric_name, metric_value, metric_type)
    VALUES ('metrics_cleanup_deleted_count', deleted_count, 'counter');
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to clean up old health checks
CREATE OR REPLACE FUNCTION cleanup_old_health_checks(retention_days INTEGER DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM health_checks 
    WHERE checked_at < CURRENT_TIMESTAMP - (retention_days || ' days')::INTERVAL
    AND environment = 'production';
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    -- Log cleanup operation
    INSERT INTO app_metrics (metric_name, metric_value, metric_type)
    VALUES ('health_checks_cleanup_deleted_count', deleted_count, 'counter');
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to clean up old error logs
CREATE OR REPLACE FUNCTION cleanup_old_error_logs(retention_days INTEGER DEFAULT 90)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM error_logs 
    WHERE occurred_at < CURRENT_TIMESTAMP - (retention_days || ' days')::INTERVAL;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    INSERT INTO app_metrics (metric_name, metric_value, metric_type)
    VALUES ('error_logs_cleanup_deleted_count', deleted_count, 'counter');
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to clean up old audit records
CREATE OR REPLACE FUNCTION cleanup_old_audit_trail(retention_days INTEGER DEFAULT 365)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM audit_trail 
    WHERE changed_at < CURRENT_TIMESTAMP - (retention_days || ' days')::INTERVAL;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    INSERT INTO app_metrics (metric_name, metric_value, metric_type)
    VALUES ('audit_trail_cleanup_deleted_count', deleted_count, 'counter');
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- =============================================================================
-- PRODUCTION MONITORING VIEWS
-- =============================================================================

-- Production health summary view
CREATE OR REPLACE VIEW production_health_summary AS
SELECT 
    service_name,
    status,
    COUNT(*) as status_count,
    MAX(checked_at) as last_check,
    AVG(CASE WHEN status = 'healthy' THEN 1 ELSE 0 END) as health_ratio
FROM health_checks 
WHERE environment = 'production' 
    AND checked_at > CURRENT_TIMESTAMP - INTERVAL '1 hour'
GROUP BY service_name, status
ORDER BY service_name, status;

-- Recent error summary view  
CREATE OR REPLACE VIEW recent_errors_summary AS
SELECT 
    error_level,
    service_name,
    COUNT(*) as error_count,
    MAX(occurred_at) as last_occurrence
FROM error_logs 
WHERE occurred_at > CURRENT_TIMESTAMP - INTERVAL '24 hours'
GROUP BY error_level, service_name
ORDER BY error_count DESC, last_occurrence DESC;

-- =============================================================================
-- PRODUCTION SECURITY
-- =============================================================================

-- Create limited read-only role for monitoring
CREATE ROLE IF NOT EXISTS radarr_monitor;
GRANT CONNECT ON DATABASE radarr TO radarr_monitor;
GRANT USAGE ON SCHEMA public TO radarr_monitor;

-- Grant read-only access to monitoring tables
GRANT SELECT ON health_checks TO radarr_monitor;
GRANT SELECT ON app_metrics TO radarr_monitor;
GRANT SELECT ON error_logs TO radarr_monitor;
GRANT SELECT ON production_health_summary TO radarr_monitor;
GRANT SELECT ON recent_errors_summary TO radarr_monitor;

-- Deny access to sensitive tables
REVOKE ALL ON audit_trail FROM radarr_monitor;

-- =============================================================================
-- PRODUCTION TABLE MAINTENANCE
-- =============================================================================

-- Enable auto-vacuum for production tables
ALTER TABLE health_checks SET (
    autovacuum_vacuum_scale_factor = 0.1,
    autovacuum_analyze_scale_factor = 0.05
);

ALTER TABLE app_metrics SET (
    autovacuum_vacuum_scale_factor = 0.1,
    autovacuum_analyze_scale_factor = 0.05
);

ALTER TABLE error_logs SET (
    autovacuum_vacuum_scale_factor = 0.2,
    autovacuum_analyze_scale_factor = 0.1
);

-- =============================================================================
-- PRODUCTION TRIGGERS
-- =============================================================================

-- Trigger to automatically record metric when error is logged
CREATE OR REPLACE FUNCTION record_error_metric()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO app_metrics (metric_name, metric_value, metric_type, labels)
    VALUES (
        'errors_total',
        1,
        'counter',
        jsonb_build_object(
            'level', NEW.error_level,
            'service', NEW.service_name
        )
    );
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER error_metric_trigger
    AFTER INSERT ON error_logs
    FOR EACH ROW
    EXECUTE FUNCTION record_error_metric();

-- =============================================================================
-- INITIAL PRODUCTION SETUP
-- =============================================================================

-- Record initial production setup
INSERT INTO health_checks (service_name, status, details, environment) 
VALUES (
    'database_init', 
    'healthy', 
    jsonb_build_object(
        'message', 'Production database initialized successfully',
        'version', '1.0.0-prod',
        'timestamp', CURRENT_TIMESTAMP,
        'extensions', ARRAY['uuid-ossp', 'pg_trgm', 'btree_gin']
    ),
    'production'
);

-- Record initial metrics
INSERT INTO app_metrics (metric_name, metric_value, metric_type) VALUES
('database_initialization', 1, 'counter'),
('schema_version', 1.0, 'gauge'),
('tables_created', 6, 'gauge');

-- =============================================================================
-- PRODUCTION VACUUM AND ANALYZE
-- =============================================================================

-- Perform initial vacuum and analyze for optimal performance
VACUUM (ANALYZE, VERBOSE);

-- Update statistics for query planner
ANALYZE;

-- =============================================================================
-- COMPLETION LOG
-- =============================================================================

DO $$
BEGIN
    RAISE LOG 'Radarr MVP production database initialization completed successfully';
    RAISE LOG 'Database: %, Environment: production', current_database();
    RAISE LOG 'Security: monitoring role created, audit trail enabled';
    RAISE LOG 'Monitoring: health checks, metrics, and error logging configured';
    
    -- Record completion metric
    INSERT INTO app_metrics (metric_name, metric_value, metric_type)
    VALUES ('database_initialization_completed', 1, 'counter');
END
$$;