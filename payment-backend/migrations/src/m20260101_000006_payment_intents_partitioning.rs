//! Migration M006 — Payment Intents Partitioning
//!
//! Converts payment_intents to a partitioned table for billion-row scale.
//! Uses range partitioning on created_at for efficient time-range queries
//! and archival.
//!
//! ## Strategy
//! - Monthly partitions for current year + 1 year ahead
//! - Partition maintenance functions for automated creation
//! - Indexes on partitioned table (inherited by all partitions)
//!
//! ## Notes
//! - This migration creates a NEW partitioned table and migrates data
//! - The old table is renamed for safety (can be dropped later)
//! - Partition maintenance functions enable automated partition creation

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Create partition maintenance functions
        self.create_partition_functions(manager).await?;

        // Step 2: Create the partitioned table
        self.create_partitioned_table(manager).await?;

        // Step 3: Create monthly partitions for 2024-2027
        self.create_monthly_partitions(manager).await?;

        // Step 4: Create indexes on partitioned table
        self.create_indexes(manager).await?;

        // Step 5: Migrate data from old table (if it exists)
        self.migrate_data(manager).await?;

        // Step 6: Rename old table for safety
        self.rename_old_table(manager).await?;

        // Step 7: Rename partitioned table back to payment_intents
        // This allows the application code to use the same table name
        self.rename_partitioned_table(manager).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Step 1: Rename current payment_intents to payment_intents_partitioned
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "ALTER TABLE IF EXISTS payment_intents RENAME TO payment_intents_partitioned;".to_string(),
        ))
        .await?;

        // Step 2: Rename old table back if it exists
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "ALTER TABLE IF EXISTS payment_intents_old RENAME TO payment_intents;".to_string(),
        ))
        .await?;

        // Step 3: Drop partitioned table and functions
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "DROP TABLE IF EXISTS payment_intents_partitioned CASCADE;".to_string(),
        ))
        .await?;

        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "DROP FUNCTION IF EXISTS create_monthly_partition CASCADE;".to_string(),
        ))
        .await?;

        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "DROP FUNCTION IF EXISTS drop_old_partitions CASCADE;".to_string(),
        ))
        .await?;

        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "DROP FUNCTION IF EXISTS get_next_partition_date CASCADE;".to_string(),
        ))
        .await?;

        Ok(())
    }
}

impl Migration {
    /// Create partition maintenance functions.
    async fn create_partition_functions(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Function to create a new monthly partition
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            r#"
            CREATE OR REPLACE FUNCTION create_monthly_partition(
                p_table_name TEXT,
                p_start_date DATE
            ) RETURNS VOID AS $$
            DECLARE
                v_end_date DATE;
                v_partition_name TEXT;
            BEGIN
                v_end_date := p_start_date + INTERVAL '1 month';
                v_partition_name := p_table_name || '_' || TO_CHAR(p_start_date, 'YYYY_MM');
                
                EXECUTE FORMAT(
                    'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)',
                    v_partition_name,
                    p_table_name,
                    p_start_date,
                    v_end_date
                );
                
                RAISE NOTICE 'Created partition: %', v_partition_name;
            END;
            $$ LANGUAGE plpgsql;
            "#
            .to_string(),
        ))
        .await?;

        // Function to drop old partitions
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            r#"
            CREATE OR REPLACE FUNCTION drop_old_partitions(
                p_table_name TEXT,
                p_retention_days INTEGER DEFAULT 365
            ) RETURNS VOID AS $$
            DECLARE
                v_cutoff_date DATE;
                v_partition_record RECORD;
            BEGIN
                v_cutoff_date := CURRENT_DATE - (p_retention_days || ' days')::INTERVAL;
                
                FOR v_partition_record IN
                    SELECT schemaname, tablename 
                    FROM pg_tables 
                    WHERE tablename LIKE p_table_name || '_20%'
                    AND tablename < p_table_name || '_' || TO_CHAR(v_cutoff_date, 'YYYY_MM')
                LOOP
                    EXECUTE FORMAT('DROP TABLE IF EXISTS %I.%I', 
                        v_partition_record.schemaname, 
                        v_partition_record.tablename);
                    RAISE NOTICE 'Dropped partition: %.%', 
                        v_partition_record.schemaname, 
                        v_partition_record.tablename;
                END LOOP;
            END;
            $$ LANGUAGE plpgsql;
            "#
            .to_string(),
        ))
        .await?;

        // Function to get next partition date
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            r#"
            CREATE OR REPLACE FUNCTION get_next_partition_date(
                p_table_name TEXT
            ) RETURNS DATE AS $$
            DECLARE
                v_max_date DATE;
            BEGIN
                SELECT MAX(
                    CASE 
                        WHEN tablename ~ p_table_name || '_[0-9]{4}_[0-9]{2}$' THEN
                            TO_DATE(SUBSTRING(tablename FROM '_([0-9]{4}_[0-9]{2})$'), 'YYYY_MM')
                    END
                ) INTO v_max_date
                FROM pg_tables
                WHERE tablename LIKE p_table_name || '_%';
                
                IF v_max_date IS NULL THEN
                    RETURN DATE_TRUNC('month', CURRENT_DATE);
                ELSE
                    RETURN v_max_date + INTERVAL '1 month';
                END IF;
            END;
            $$ LANGUAGE plpgsql;
            "#
            .to_string(),
        ))
        .await?;

        Ok(())
    }

    /// Create the partitioned table.
    async fn create_partitioned_table(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            r#"
            CREATE TABLE IF NOT EXISTS payment_intents_partitioned (
                payment_intent_id UUID PRIMARY KEY,
                operator_id UUID NOT NULL,
                status VARCHAR(32) NOT NULL,
                amount_minor_units BIGINT NOT NULL,
                currency VARCHAR(3) NOT NULL,
                idempotency_key VARCHAR(255) NOT NULL,
                purpose VARCHAR(32) NOT NULL DEFAULT 'payment',
                source_type VARCHAR(32),
                source_id UUID,
                payment_method_token_id UUID,
                routing_policy_id UUID,
                gateway_profile_id UUID,
                risk_score DOUBLE PRECISION,
                risk_level VARCHAR(16),
                metadata_json JSONB,
                routing_attempts JSONB NOT NULL DEFAULT '[]'::jsonb,
                authorized_amount_minor BIGINT NOT NULL DEFAULT 0,
                captured_amount_minor BIGINT NOT NULL DEFAULT 0,
                refunded_amount_minor BIGINT NOT NULL DEFAULT 0,
                version BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            ) PARTITION BY RANGE (created_at);
            "#
            .to_string(),
        ))
        .await?;

        Ok(())
    }

    /// Create monthly partitions for 2024-2027.
    async fn create_monthly_partitions(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Create partitions for each month from 2024-01 to 2027-12
        for year in 2024..=2027 {
            for month in 1..=12 {
                let start_date = format!("{}-{:02}-01", year, month);
                let end_date = if month == 12 {
                    format!("{}-01-01", year + 1)
                } else {
                    format!("{}-{:02}-01", year, month + 1)
                };

                conn.execute(sea_orm::Statement::from_string(
                    manager.get_database_backend(),
                    format!(
                        r#"
                        CREATE TABLE IF NOT EXISTS payment_intents_{year:04}_{month:02} 
                        PARTITION OF payment_intents_partitioned
                        FOR VALUES FROM ('{start_date}') TO ('{end_date}');
                        "#,
                        year = year,
                        month = month,
                        start_date = start_date,
                        end_date = end_date
                    ),
                ))
                .await?;
            }
        }

        Ok(())
    }

    /// Create indexes on the partitioned table.
    async fn create_indexes(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Operator ID index (for tenant isolation queries)
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "CREATE INDEX IF NOT EXISTS idx_pi_p_operator ON payment_intents_partitioned (operator_id);".to_string(),
        ))
        .await?;

        // Status index (for filtering by status)
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "CREATE INDEX IF NOT EXISTS idx_pi_p_status ON payment_intents_partitioned (status);".to_string(),
        ))
        .await?;

        // Created at index (for time-range queries)
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "CREATE INDEX IF NOT EXISTS idx_pi_p_created ON payment_intents_partitioned (created_at);".to_string(),
        ))
        .await?;

        // Idempotency key index (for deduplication)
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_pi_p_idempotency ON payment_intents_partitioned (idempotency_key);".to_string(),
        ))
        .await?;

        // Gateway profile ID index (for routing analytics)
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "CREATE INDEX IF NOT EXISTS idx_pi_p_gateway ON payment_intents_partitioned (gateway_profile_id);".to_string(),
        ))
        .await?;

        // Composite index for common query patterns
        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "CREATE INDEX IF NOT EXISTS idx_pi_p_operator_status ON payment_intents_partitioned (operator_id, status);".to_string(),
        ))
        .await?;

        Ok(())
    }

    /// Migrate data from old table to partitioned table.
    async fn migrate_data(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Check if old table exists and has data
        let check = conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'payment_intents')".to_string(),
        ))
        .await?;

        // Only migrate if old table exists
        if check.rows_affected() > 0 {
            conn.execute(sea_orm::Statement::from_string(
                manager.get_database_backend(),
                r#"
                INSERT INTO payment_intents_partitioned (
                    payment_intent_id, operator_id, status, amount_minor_units, currency,
                    idempotency_key, purpose, source_type, source_id, payment_method_token_id,
                    routing_policy_id, gateway_profile_id, risk_score, risk_level,
                    metadata_json, routing_attempts, authorized_amount_minor,
                    captured_amount_minor, refunded_amount_minor, version, created_at, updated_at
                )
                SELECT 
                    payment_intent_id, operator_id, status, amount_minor_units, currency,
                    idempotency_key, purpose, source_type, source_id, payment_method_token_id,
                    routing_policy_id, gateway_profile_id, risk_score, risk_level,
                    metadata_json, routing_attempts, authorized_amount_minor,
                    captured_amount_minor, refunded_amount_minor, version, created_at, updated_at
                FROM payment_intents
                ON CONFLICT (payment_intent_id) DO NOTHING;
                "#
                .to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    /// Rename old table for safety.
    async fn rename_old_table(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // Check if old table exists
        let check = conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'payment_intents')".to_string(),
        ))
        .await?;

        if check.rows_affected() > 0 {
            conn.execute(sea_orm::Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE IF EXISTS payment_intents RENAME TO payment_intents_old;".to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    /// Rename partitioned table back to payment_intents.
    /// This allows the application code to use the same table name.
    async fn rename_partitioned_table(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "ALTER TABLE IF EXISTS payment_intents_partitioned RENAME TO payment_intents;".to_string(),
        ))
        .await?;

        Ok(())
    }
}
