-- Fix migration conflicts by using IF NOT EXISTS
-- This migration ensures all required tables and indexes exist without conflicts

-- Ensure dead letter queue table exists
CREATE TABLE IF NOT EXISTS dead_letter_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_id UUID NOT NULL,
    original_type VARCHAR(50) NOT NULL,
    failure_count INTEGER NOT NULL DEFAULT 1,
    first_failed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_failed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_error TEXT NOT NULL,
    error_history JSONB DEFAULT '[]'::jsonb,
    item_data JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'failed',
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolution_notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Ensure queue table has required columns
ALTER TABLE queue ADD COLUMN IF NOT EXISTS retry_count INTEGER DEFAULT 0;
ALTER TABLE queue ADD COLUMN IF NOT EXISTS last_retry_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE queue ADD COLUMN IF NOT EXISTS max_retries INTEGER DEFAULT 3;

-- Create indexes only if they don't exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_dead_letter_queue_status') THEN
        CREATE INDEX idx_dead_letter_queue_status ON dead_letter_queue(status);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_dead_letter_queue_original_type') THEN
        CREATE INDEX idx_dead_letter_queue_original_type ON dead_letter_queue(original_type);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_dead_letter_queue_last_failed_at') THEN
        CREATE INDEX idx_dead_letter_queue_last_failed_at ON dead_letter_queue(last_failed_at);
    END IF;
END
$$;