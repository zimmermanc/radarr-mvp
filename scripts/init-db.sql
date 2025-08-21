-- Database initialization script for development
-- This script is run when the PostgreSQL container starts for the first time

-- Ensure the database is created with proper encoding
CREATE DATABASE radarr_dev WITH 
    ENCODING 'UTF8' 
    LC_COLLATE='C' 
    LC_CTYPE='C' 
    TEMPLATE template0;

-- Connect to the database
\c radarr_dev;

-- Create extensions that might be needed
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Grant privileges to the radarr user
GRANT ALL PRIVILEGES ON DATABASE radarr_dev TO radarr;
GRANT ALL ON SCHEMA public TO radarr;

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO radarr;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO radarr;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO radarr;