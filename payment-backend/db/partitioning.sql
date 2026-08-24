-- =============================================================================
-- PostgreSQL Table Partitioning for Payment Orchestration Platform
-- =============================================================================
-- This file contains partitioning strategies for high-volume tables.
-- Run this migration to enable horizontal partitioning.

-- ─── Payment Intents Partitioning ────────────────────────────────────────────
-- Partition by created_at for efficient time-range queries and archival.

-- First, create the partitioned table (if not exists)
CREATE TABLE IF NOT EXISTS payment_intents_partitioned (
    payment_intent_id UUID PRIMARY KEY,
    operator_id UUID NOT NULL,
    status VARCHAR(50) NOT NULL,
    requested_amount_minor BIGINT NOT NULL DEFAULT 0,
    authorized_amount_minor BIGINT NOT NULL DEFAULT 0,
    captured_amount_minor BIGINT NOT NULL DEFAULT 0,
    refunded_amount_minor BIGINT NOT NULL DEFAULT 0,
    currency VARCHAR(3) NOT NULL DEFAULT 'AED',
    idempotency_key VARCHAR(255) NOT NULL,
    payment_method_token_id UUID,
    routing_policy_id UUID,
    deployment_epoch INTEGER NOT NULL DEFAULT 0,
    purpose VARCHAR(50) NOT NULL DEFAULT 'Payment',
    metadata JSONB,
    source_type VARCHAR(50),
    source_id VARCHAR(255),
    risk_score DECIMAL(5,2),
    risk_level VARCHAR(20),
    expected_settlement_date TIMESTAMPTZ,
    settlement_cycle VARCHAR(50),
    gateway_profile_id UUID,
    gateway_profile_version INTEGER,
    gateway_rotation_strategy VARCHAR(50),
    gateway_selection_reason TEXT,
    routing_attempts JSONB DEFAULT '[]'::jsonb,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Create monthly partitions for current year
CREATE TABLE payment_intents_2024_01 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
CREATE TABLE payment_intents_2024_02 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-02-01') TO ('2024-03-01');
CREATE TABLE payment_intents_2024_03 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-03-01') TO ('2024-04-01');
CREATE TABLE payment_intents_2024_04 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-04-01') TO ('2024-05-01');
CREATE TABLE payment_intents_2024_05 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-05-01') TO ('2024-06-01');
CREATE TABLE payment_intents_2024_06 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-06-01') TO ('2024-07-01');
CREATE TABLE payment_intents_2024_07 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-07-01') TO ('2024-08-01');
CREATE TABLE payment_intents_2024_08 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-08-01') TO ('2024-09-01');
CREATE TABLE payment_intents_2024_09 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-09-01') TO ('2024-10-01');
CREATE TABLE payment_intents_2024_10 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-10-01') TO ('2024-11-01');
CREATE TABLE payment_intents_2024_11 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-11-01') TO ('2024-12-01');
CREATE TABLE payment_intents_2024_12 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2024-12-01') TO ('2025-01-01');

-- Create monthly partitions for 2025
CREATE TABLE payment_intents_2025_01 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
CREATE TABLE payment_intents_2025_02 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
CREATE TABLE payment_intents_2025_03 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');
CREATE TABLE payment_intents_2025_04 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-04-01') TO ('2025-05-01');
CREATE TABLE payment_intents_2025_05 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-05-01') TO ('2025-06-01');
CREATE TABLE payment_intents_2025_06 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-06-01') TO ('2025-07-01');
CREATE TABLE payment_intents_2025_07 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-07-01') TO ('2025-08-01');
CREATE TABLE payment_intents_2025_08 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-08-01') TO ('2025-09-01');
CREATE TABLE payment_intents_2025_09 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');
CREATE TABLE payment_intents_2025_10 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');
CREATE TABLE payment_intents_2025_11 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
CREATE TABLE payment_intents_2025_12 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- Create monthly partitions for 2026
CREATE TABLE payment_intents_2026_01 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE payment_intents_2026_02 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE payment_intents_2026_03 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE payment_intents_2026_04 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE payment_intents_2026_05 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE payment_intents_2026_06 PARTITION OF payment_intents_partitioned
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

-- ─── Indexes for Partitioned Table ───────────────────────────────────────────
-- Create indexes on the partitioned table (inherited by all partitions)

CREATE INDEX idx_payment_intents_operator_id ON payment_intents_partitioned (operator_id);
CREATE INDEX idx_payment_intents_status ON payment_intents_partitioned (status);
CREATE INDEX idx_payment_intents_created_at ON payment_intents_partitioned (created_at);
CREATE INDEX idx_payment_intents_idempotency_key ON payment_intents_partitioned (idempotency_key);
CREATE INDEX idx_payment_intents_gateway_profile_id ON payment_intents_partitioned (gateway_profile_id);

-- ─── Stored Events Partitioning ──────────────────────────────────────────────
-- Partition by occurred_at for efficient event replay and archival.

CREATE TABLE IF NOT EXISTS stored_events_partitioned (
    event_id UUID PRIMARY KEY,
    aggregate_type VARCHAR(100) NOT NULL,
    aggregate_id UUID NOT NULL,
    event_type VARCHAR(200) NOT NULL,
    event_sequence BIGINT NOT NULL,
    event_version INTEGER NOT NULL DEFAULT 1,
    occurred_at TIMESTAMPTZ NOT NULL,
    actor_type VARCHAR(50) NOT NULL DEFAULT 'system',
    actor_id UUID,
    causation_id UUID,
    correlation_id UUID,
    payload BYTEA NOT NULL,
    encrypted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (occurred_at);

-- Create monthly partitions for stored events
CREATE TABLE stored_events_2024_01 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
CREATE TABLE stored_events_2024_02 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-02-01') TO ('2024-03-01');
CREATE TABLE stored_events_2024_03 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-03-01') TO ('2024-04-01');
CREATE TABLE stored_events_2024_04 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-04-01') TO ('2024-05-01');
CREATE TABLE stored_events_2024_05 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-05-01') TO ('2024-06-01');
CREATE TABLE stored_events_2024_06 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-06-01') TO ('2024-07-01');
CREATE TABLE stored_events_2024_07 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-07-01') TO ('2024-08-01');
CREATE TABLE stored_events_2024_08 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-08-01') TO ('2024-09-01');
CREATE TABLE stored_events_2024_09 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-09-01') TO ('2024-10-01');
CREATE TABLE stored_events_2024_10 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-10-01') TO ('2024-11-01');
CREATE TABLE stored_events_2024_11 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-11-01') TO ('2024-12-01');
CREATE TABLE stored_events_2024_12 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2024-12-01') TO ('2025-01-01');

-- Create monthly partitions for 2025
CREATE TABLE stored_events_2025_01 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
CREATE TABLE stored_events_2025_02 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
CREATE TABLE stored_events_2025_03 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');
CREATE TABLE stored_events_2025_04 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-04-01') TO ('2025-05-01');
CREATE TABLE stored_events_2025_05 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-05-01') TO ('2025-06-01');
CREATE TABLE stored_events_2025_06 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-06-01') TO ('2025-07-01');
CREATE TABLE stored_events_2025_07 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-07-01') TO ('2025-08-01');
CREATE TABLE stored_events_2025_08 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-08-01') TO ('2025-09-01');
CREATE TABLE stored_events_2025_09 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');
CREATE TABLE stored_events_2025_10 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');
CREATE TABLE stored_events_2025_11 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
CREATE TABLE stored_events_2025_12 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- Create monthly partitions for 2026
CREATE TABLE stored_events_2026_01 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE stored_events_2026_02 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE stored_events_2026_03 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE stored_events_2026_04 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE stored_events_2026_05 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE stored_events_2026_06 PARTITION OF stored_events_partitioned
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

-- ─── Indexes for Stored Events ───────────────────────────────────────────────

CREATE INDEX idx_stored_events_aggregate ON stored_events_partitioned (aggregate_type, aggregate_id);
CREATE INDEX idx_stored_events_event_type ON stored_events_partitioned (event_type);
CREATE INDEX idx_stored_events_occurred_at ON stored_events_partitioned (occurred_at);
CREATE INDEX idx_stored_events_correlation ON stored_events_partitioned (correlation_id);

-- ─── Partition Maintenance Function ──────────────────────────────────────────
-- Function to create new partitions automatically

CREATE OR REPLACE FUNCTION create_monthly_partition(
    table_name TEXT,
    start_date DATE
) RETURNS VOID AS $$
DECLARE
    end_date DATE;
    partition_name TEXT;
BEGIN
    end_date := start_date + INTERVAL '1 month';
    partition_name := table_name || '_' || TO_CHAR(start_date, 'YYYY_MM');
    
    EXECUTE FORMAT(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)',
        partition_name,
        table_name,
        start_date,
        end_date
    );
END;
$$ LANGUAGE plpgsql;

-- Function to drop old partitions (for archival)
CREATE OR REPLACE FUNCTION drop_old_partitions(
    table_name TEXT,
    retention_days INTEGER DEFAULT 365
) RETURNS VOID AS $$
DECLARE
    cutoff_date DATE;
    partition_record RECORD;
BEGIN
    cutoff_date := CURRENT_DATE - (retention_days || ' days')::INTERVAL;
    
    FOR partition_record IN
        SELECT schemaname, tablename 
        FROM pg_tables 
        WHERE tablename LIKE table_name || '_20%'
        AND tablename < table_name || '_' || TO_CHAR(cutoff_date, 'YYYY_MM')
    LOOP
        EXECUTE FORMAT('DROP TABLE IF EXISTS %I.%I', 
            partition_record.schemaname, 
            partition_record.tablename);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- ─── Comments ────────────────────────────────────────────────────────────────

COMMENT ON TABLE payment_intents_partitioned IS 'Payment intents with monthly partitioning for performance';
COMMENT ON TABLE stored_events_partitioned IS 'Event store with monthly partitioning for event sourcing';
COMMENT ON FUNCTION create_monthly_partition IS 'Creates a new monthly partition for a table';
COMMENT ON FUNCTION drop_old_partitions IS 'Drops partitions older than retention period';
