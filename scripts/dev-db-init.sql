-- =============================================================================
-- Development Database Initialization Script for Radarr MVP
-- =============================================================================
-- This script runs in development mode with additional debugging and test data

-- Connect to the development database
\c radarr_dev;

-- =============================================================================
-- DEVELOPMENT-SPECIFIC EXTENSIONS
-- =============================================================================

-- Enable additional extensions useful for development
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";  -- Query performance tracking

-- =============================================================================
-- DEVELOPMENT CONFIGURATION
-- =============================================================================

-- More verbose logging for development
SET log_statement = 'all';
SET log_min_duration_statement = 0;  -- Log all queries regardless of duration
SET log_line_prefix = '%t [%p]: [%l-1] user=%u,db=%d,app=%a,client=%h ';

-- =============================================================================
-- DEVELOPMENT SCHEMA INFO
-- =============================================================================

CREATE TABLE IF NOT EXISTS schema_info (
    version VARCHAR(50) PRIMARY KEY,
    description TEXT,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    environment VARCHAR(20) DEFAULT 'development'
);

-- Insert development schema version
INSERT INTO schema_info (version, description, environment) 
VALUES ('1.0.0-dev', 'Development Radarr MVP database setup', 'development')
ON CONFLICT (version) DO NOTHING;

-- =============================================================================
-- DEVELOPMENT HEALTH CHECKS
-- =============================================================================

CREATE TABLE IF NOT EXISTS health_checks (
    id SERIAL PRIMARY KEY,
    service_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('healthy', 'unhealthy', 'degraded')),
    details JSONB,
    checked_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    environment VARCHAR(20) DEFAULT 'development'
);

CREATE INDEX IF NOT EXISTS idx_health_checks_service_time 
ON health_checks(service_name, checked_at DESC);

-- =============================================================================
-- DEVELOPMENT TEST DATA
-- =============================================================================

-- Create test data tables for development
CREATE TABLE IF NOT EXISTS test_movies (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    year INTEGER,
    imdb_id VARCHAR(20),
    tmdb_id INTEGER,
    status VARCHAR(50) DEFAULT 'wanted',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Insert some test movies for development
INSERT INTO test_movies (title, year, imdb_id, tmdb_id, status) VALUES
('The Matrix', 1999, 'tt0133093', 603, 'downloaded'),
('Inception', 2010, 'tt1375666', 27205, 'wanted'),
('Interstellar', 2014, 'tt0816692', 157336, 'wanted'),
('The Dark Knight', 2008, 'tt0468569', 155, 'downloaded'),
('Pulp Fiction', 1994, 'tt0110912', 680, 'downloaded')
ON CONFLICT DO NOTHING;

-- Create test releases table
CREATE TABLE IF NOT EXISTS test_releases (
    id SERIAL PRIMARY KEY,
    title VARCHAR(500) NOT NULL,
    size_bytes BIGINT,
    seeders INTEGER DEFAULT 0,
    leechers INTEGER DEFAULT 0,
    quality VARCHAR(50),
    source VARCHAR(100),
    movie_id INTEGER REFERENCES test_movies(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Insert test releases
INSERT INTO test_releases (title, size_bytes, seeders, leechers, quality, source, movie_id) VALUES
('The.Matrix.1999.2160p.BluRay.x265-EXAMPLE', 15000000000, 150, 5, '2160p', 'BluRay', 1),
('The.Matrix.1999.1080p.BluRay.x264-EXAMPLE', 8000000000, 300, 10, '1080p', 'BluRay', 1),
('Inception.2010.2160p.WEB-DL.x265-EXAMPLE', 12000000000, 80, 3, '2160p', 'WEB-DL', 2),
('Interstellar.2014.1080p.BluRay.x264-EXAMPLE', 10000000000, 200, 8, '1080p', 'BluRay', 3),
('The.Dark.Knight.2008.2160p.UHD.BluRay.x265-EXAMPLE', 18000000000, 120, 4, '2160p', 'BluRay', 4)
ON CONFLICT DO NOTHING;

-- =============================================================================
-- DEVELOPMENT UTILITIES
-- =============================================================================

-- Create a function to reset test data
CREATE OR REPLACE FUNCTION reset_test_data()
RETURNS VOID AS $$
BEGIN
    DELETE FROM test_releases;
    DELETE FROM test_movies;
    DELETE FROM health_checks WHERE environment = 'development';
    
    -- Re-insert test data
    INSERT INTO test_movies (title, year, imdb_id, tmdb_id, status) VALUES
    ('The Matrix', 1999, 'tt0133093', 603, 'downloaded'),
    ('Inception', 2010, 'tt1375666', 27205, 'wanted'),
    ('Interstellar', 2014, 'tt0816692', 157336, 'wanted'),
    ('The Dark Knight', 2008, 'tt0468569', 155, 'downloaded'),
    ('Pulp Fiction', 1994, 'tt0110912', 680, 'downloaded');
    
    INSERT INTO test_releases (title, size_bytes, seeders, leechers, quality, source, movie_id) VALUES
    ('The.Matrix.1999.2160p.BluRay.x265-EXAMPLE', 15000000000, 150, 5, '2160p', 'BluRay', 1),
    ('The.Matrix.1999.1080p.BluRay.x264-EXAMPLE', 8000000000, 300, 10, '1080p', 'BluRay', 1),
    ('Inception.2010.2160p.WEB-DL.x265-EXAMPLE', 12000000000, 80, 3, '2160p', 'WEB-DL', 2),
    ('Interstellar.2014.1080p.BluRay.x264-EXAMPLE', 10000000000, 200, 8, '1080p', 'BluRay', 3),
    ('The.Dark.Knight.2008.2160p.UHD.BluRay.x265-EXAMPLE', 18000000000, 120, 4, '2160p', 'BluRay', 4);
    
    RAISE NOTICE 'Test data reset successfully';
END;
$$ LANGUAGE plpgsql;

-- Create a function to generate random test data
CREATE OR REPLACE FUNCTION generate_random_test_data(movie_count INTEGER DEFAULT 10)
RETURNS VOID AS $$
DECLARE
    i INTEGER;
    movie_titles TEXT[] := ARRAY[
        'Blade Runner 2049', 'Mad Max: Fury Road', 'Ex Machina', 'Her', 'Ghost in the Shell',
        'Akira', 'The Terminator', 'Alien', 'Star Wars', 'Avatar',
        'Guardians of the Galaxy', 'Spider-Man', 'Iron Man', 'Thor', 'Captain America'
    ];
    years INTEGER[] := ARRAY[2020, 2021, 2022, 2023, 2024];
    qualities TEXT[] := ARRAY['720p', '1080p', '2160p'];
    sources TEXT[] := ARRAY['BluRay', 'WEB-DL', 'HDTV', 'CAM'];
BEGIN
    FOR i IN 1..movie_count LOOP
        INSERT INTO test_movies (title, year, status) VALUES (
            movie_titles[1 + (random() * (array_length(movie_titles, 1) - 1))::INTEGER],
            years[1 + (random() * (array_length(years, 1) - 1))::INTEGER],
            CASE WHEN random() > 0.5 THEN 'wanted' ELSE 'downloaded' END
        );
    END LOOP;
    
    RAISE NOTICE 'Generated % random test movies', movie_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- DEVELOPMENT MONITORING
-- =============================================================================

-- Create a view for development monitoring
CREATE OR REPLACE VIEW dev_stats AS
SELECT 
    'movies' as table_name,
    COUNT(*) as row_count,
    COUNT(*) FILTER (WHERE status = 'wanted') as wanted_count,
    COUNT(*) FILTER (WHERE status = 'downloaded') as downloaded_count
FROM test_movies
UNION ALL
SELECT 
    'releases' as table_name,
    COUNT(*) as row_count,
    AVG(size_bytes)::BIGINT as avg_size,
    AVG(seeders)::INTEGER as avg_seeders
FROM test_releases
UNION ALL
SELECT
    'health_checks' as table_name,
    COUNT(*) as row_count,
    COUNT(*) FILTER (WHERE status = 'healthy') as healthy_count,
    COUNT(*) FILTER (WHERE status != 'healthy') as unhealthy_count
FROM health_checks;

-- =============================================================================
-- DEVELOPMENT PERMISSIONS
-- =============================================================================

-- Grant permissions to development user
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO radarr_dev;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO radarr_dev;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO radarr_dev;

-- Set default permissions for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO radarr_dev;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO radarr_dev;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO radarr_dev;

-- =============================================================================
-- INITIAL DEVELOPMENT HEALTH CHECK
-- =============================================================================

INSERT INTO health_checks (service_name, status, details, environment) 
VALUES (
    'database_init', 
    'healthy', 
    jsonb_build_object(
        'message', 'Development database initialized successfully',
        'test_movies_count', (SELECT COUNT(*) FROM test_movies),
        'test_releases_count', (SELECT COUNT(*) FROM test_releases),
        'timestamp', CURRENT_TIMESTAMP
    ),
    'development'
);

-- =============================================================================
-- VACUUM AND ANALYZE
-- =============================================================================

VACUUM ANALYZE;

-- =============================================================================
-- COMPLETION LOG
-- =============================================================================

DO $$
BEGIN
    RAISE LOG 'Radarr MVP development database initialization completed';
    RAISE LOG 'Database: %, User: %, Environment: development', current_database(), current_user;
    RAISE LOG 'Test data: % movies, % releases', 
              (SELECT COUNT(*) FROM test_movies), 
              (SELECT COUNT(*) FROM test_releases);
END
$$;