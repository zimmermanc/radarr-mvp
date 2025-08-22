-- Add dead letter queue table for permanently failed items

-- Create dead letter queue table
CREATE TABLE IF NOT EXISTS dead_letter_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Original item details
    original_id UUID NOT NULL,
    original_type VARCHAR(50) NOT NULL, -- 'download', 'import', 'indexer_search', etc
    
    -- Failure details
    failure_count INTEGER NOT NULL DEFAULT 1,
    first_failed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_failed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_error TEXT NOT NULL,
    error_history JSONB DEFAULT '[]'::jsonb, -- Array of {timestamp, error, context}
    
    -- Item data
    item_data JSONB NOT NULL, -- Complete original item for potential retry
    
    -- Resolution tracking
    status VARCHAR(50) NOT NULL DEFAULT 'failed', -- 'failed', 'retrying', 'resolved', 'ignored'
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolution_notes TEXT,
    
    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX idx_dead_letter_queue_status ON dead_letter_queue(status);
CREATE INDEX idx_dead_letter_queue_original_type ON dead_letter_queue(original_type);
CREATE INDEX idx_dead_letter_queue_last_failed_at ON dead_letter_queue(last_failed_at);

-- Add retry attempts tracking to queue_items
ALTER TABLE queue_items ADD COLUMN IF NOT EXISTS retry_count INTEGER DEFAULT 0;
ALTER TABLE queue_items ADD COLUMN IF NOT EXISTS last_retry_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE queue_items ADD COLUMN IF NOT EXISTS max_retries INTEGER DEFAULT 3;

-- Function to move failed items to dead letter queue
CREATE OR REPLACE FUNCTION move_to_dead_letter_queue(
    p_original_id UUID,
    p_original_type VARCHAR(50),
    p_error TEXT,
    p_item_data JSONB
) RETURNS UUID AS $$
DECLARE
    v_dead_letter_id UUID;
BEGIN
    INSERT INTO dead_letter_queue (
        original_id,
        original_type,
        last_error,
        item_data
    ) VALUES (
        p_original_id,
        p_original_type,
        p_error,
        p_item_data
    )
    ON CONFLICT (original_id) DO UPDATE
    SET 
        failure_count = dead_letter_queue.failure_count + 1,
        last_failed_at = NOW(),
        last_error = EXCLUDED.last_error,
        error_history = dead_letter_queue.error_history || 
            jsonb_build_object(
                'timestamp', NOW(),
                'error', EXCLUDED.last_error,
                'attempt', dead_letter_queue.failure_count + 1
            ),
        updated_at = NOW()
    RETURNING id INTO v_dead_letter_id;
    
    RETURN v_dead_letter_id;
END;
$$ LANGUAGE plpgsql;

-- Add unique constraint for original_id to prevent duplicates
ALTER TABLE dead_letter_queue ADD CONSTRAINT unique_original_id UNIQUE (original_id);