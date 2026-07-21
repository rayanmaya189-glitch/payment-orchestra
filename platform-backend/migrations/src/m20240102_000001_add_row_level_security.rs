use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// RLS policies for multi-tenant data isolation (SRS SECTEST-004).
///
/// Sets `app.current_operator_id` per session, then enforces tenant-scoped
/// SELECT/INSERT/UPDATE/DELETE policies on all operator-owned tables.
///
/// platform_admin bypasses RLS via GRANT.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Create helper function to set the current operator context.
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            CREATE OR REPLACE FUNCTION set_current_operator_id(op_id TEXT)
            RETURNS VOID AS $$
            BEGIN
                PERFORM set_config('app.current_operator_id', op_id, true);
            END;
            $$ LANGUAGE plpgsql;
            "#.to_string(),
        ))
        .await?;

        // 2. Create function to get the current operator ID.
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            CREATE OR REPLACE FUNCTION get_current_operator_id()
            RETURNS UUID AS $$
            DECLARE
                op_id TEXT;
            BEGIN
                op_id := current_setting('app.current_operator_id', true);
                IF op_id IS NULL OR op_id = '' THEN
                    RETURN NULL;
                END IF;
                RETURN op_id::UUID;
            END;
            $$ LANGUAGE plpgsql;
            "#.to_string(),
        ))
        .await?;

        // 3. Enable RLS and create policies on all tenant-scoped tables.
        let tenant_tables = vec![
            "payment_intent",
            "routing_policy",
            "routing_attempt",
            "gateway_profile",
            "invoice",
            "payment_link",
            "subscription",
            "dispute",
            "notification",
            "risk_assessment",
            "settlement_batch",
            "document",
            "ai_request",
            "route_config",
            "saga_instances",
            "outbox",
            "audit_log",
            "kyb_case",
            "pending_changes",
            "refresh_tokens",
        ];

        for table in &tenant_tables {
            // Enable RLS
            db.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("ALTER TABLE {} ENABLE ROW LEVEL SECURITY;", table),
            ))
            .await?;

            // Create policy: operator-scoped access
            db.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    r#"
                    CREATE POLICY operator_isolation_policy ON {}
                    FOR ALL
                    USING (operator_id = get_current_operator_id() OR get_current_operator_id() IS NULL)
                    WITH CHECK (operator_id = get_current_operator_id() OR get_current_operator_id() IS NULL);
                    "#,
                    table
                ),
            ))
            .await?;
        }

        // 4. Grant application role full access (bypasses RLS via superuser-like grants).
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            DO $$
            BEGIN
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_role') THEN
                    CREATE ROLE app_role;
                END IF;
                GRANT USAGE ON SCHEMA public TO app_role;
                GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_role;
                ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_role;
            END
            $$;
            "#.to_string(),
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let tenant_tables = vec![
            "payment_intent",
            "routing_policy",
            "routing_attempt",
            "gateway_profile",
            "invoice",
            "payment_link",
            "subscription",
            "dispute",
            "notification",
            "risk_assessment",
            "settlement_batch",
            "document",
            "ai_request",
            "route_config",
            "saga_instances",
            "outbox",
            "audit_log",
            "kyb_case",
            "pending_changes",
            "refresh_tokens",
        ];

        for table in &tenant_tables {
            db.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("DROP POLICY IF EXISTS operator_isolation_policy ON {};", table),
            ))
            .await?;

            db.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("ALTER TABLE {} DISABLE ROW LEVEL SECURITY;", table),
            ))
            .await?;
        }

        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DROP FUNCTION IF EXISTS set_current_operator_id;".to_string(),
        ))
        .await?;

        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DROP FUNCTION IF EXISTS get_current_operator_id;".to_string(),
        ))
        .await?;

        Ok(())
    }
}
