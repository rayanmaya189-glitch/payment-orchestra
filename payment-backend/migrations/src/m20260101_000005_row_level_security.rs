//! Migration to add Row-Level Security (RLS) policies for multi-tenant isolation.
//!
//! This migration enables RLS on all tables and creates policies that ensure
//! each operator (tenant) can only access their own data.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create a function to get the current operator_id from session
        manager
            .exec_stmt(
                Query::custom_statement(
                    r#"
                    -- Create function to get current operator_id from session variable
                    CREATE OR REPLACE FUNCTION current_operator_id()
                    RETURNS UUID AS $$
                    BEGIN
                        RETURN current_setting('app.current_operator_id', true)::UUID;
                    EXCEPTION
                        WHEN OTHERS THEN
                            RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql STABLE;
                    "#,
                )
                .to_owned(),
            )
            .await?;

        // Create a function to check if user is admin (bypasses RLS)
        manager
            .exec_stmt(
                Query::custom_statement(
                    r#"
                    -- Create function to check if current user is admin
                    CREATE OR REPLACE FUNCTION is_admin_user()
                    RETURNS BOOLEAN AS $$
                    BEGIN
                        RETURN current_setting('app.is_admin', true)::BOOLEAN;
                    EXCEPTION
                        WHEN OTHERS THEN
                            RETURN FALSE;
                    END;
                    $$ LANGUAGE plpgsql STABLE;
                    "#,
                )
                .to_owned(),
            )
            .await?;

        // Enable RLS and create policies for payment_intents
        self.enable_rls_for_table(manager, "payment_intents").await?;
        self.enable_rls_for_table(manager, "routing_policies").await?;
        self.enable_rls_for_table(manager, "gateway_profiles").await?;
        self.enable_rls_for_table(manager, "merchant_acquirer_links").await?;
        self.enable_rls_for_table(manager, "invoices").await?;
        self.enable_rls_for_table(manager, "subscriptions").await?;
        self.enable_rls_for_table(manager, "reconciliation_exceptions").await?;
        self.enable_rls_for_table(manager, "webhooks").await?;
        self.enable_rls_for_table(manager, "api_keys").await?;
        self.enable_rls_for_table(manager, "audit_logs").await?;
        self.enable_rls_for_table(manager, "notification_subscriptions").await?;
        self.enable_rls_for_table(manager, "documents").await?;
        self.enable_rls_for_table(manager, "analytics_events").await?;
        self.enable_rls_for_table(manager, "risk_assessments").await?;
        self.enable_rls_for_table(manager, "dispute_cases").await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Disable RLS for all tables
        self.disable_rls_for_table(manager, "payment_intents").await?;
        self.disable_rls_for_table(manager, "routing_policies").await?;
        self.disable_rls_for_table(manager, "gateway_profiles").await?;
        self.disable_rls_for_table(manager, "merchant_acquirer_links").await?;
        self.disable_rls_for_table(manager, "invoices").await?;
        self.disable_rls_for_table(manager, "subscriptions").await?;
        self.disable_rls_for_table(manager, "reconciliation_exceptions").await?;
        self.disable_rls_for_table(manager, "webhooks").await?;
        self.disable_rls_for_table(manager, "api_keys").await?;
        self.disable_rls_for_table(manager, "audit_logs").await?;
        self.disable_rls_for_table(manager, "notification_subscriptions").await?;
        self.disable_rls_for_table(manager, "documents").await?;
        self.disable_rls_for_table(manager, "analytics_events").await?;
        self.disable_rls_for_table(manager, "risk_assessments").await?;
        self.disable_rls_for_table(manager, "dispute_cases").await?;

        // Drop functions
        manager
            .exec_stmt(Query::custom_statement("DROP FUNCTION IF EXISTS current_operator_id();").to_owned())
            .await?;
        manager
            .exec_stmt(Query::custom_statement("DROP FUNCTION IF EXISTS is_admin_user();").to_owned())
            .await?;

        Ok(())
    }
}

impl Migration {
    /// Enable RLS and create tenant isolation policy for a table.
    async fn enable_rls_for_table(&self, manager: &SchemaManager, table_name: &str) -> Result<(), DbErr> {
        // Enable RLS on the table
        manager
            .exec_stmt(
                Query::custom_statement(&format!(
                    "ALTER TABLE {} ENABLE ROW LEVEL SECURITY;",
                    table_name
                ))
                .to_owned(),
            )
            .await?;

        // Create policy for tenant isolation
        manager
            .exec_stmt(
                Query::custom_statement(&format!(
                    r#"
                    -- Policy: Operators can only access their own data
                    CREATE POLICY operator_isolation_{table} ON {table}
                        FOR ALL
                        USING (
                            is_admin_user() OR operator_id = current_operator_id()
                        );
                    "#,
                    table = table_name
                ))
                .to_owned(),
            )
            .await?;

        // Create policy for service accounts (bypass RLS for internal services)
        manager
            .exec_stmt(
                Query::custom_statement(&format!(
                    r#"
                    -- Policy: Service accounts can access all data
                    CREATE POLICY service_access_{table} ON {table}
                        FOR ALL
                        USING (
                            current_setting('app.principal_type', true) = 'service'
                        );
                    "#,
                    table = table_name
                ))
                .to_owned(),
            )
            .await?;

        Ok(())
    }

    /// Disable RLS and drop policies for a table.
    async fn disable_rls_for_table(&self, manager: &SchemaManager, table_name: &str) -> Result<(), DbErr> {
        // Drop policies
        manager
            .exec_stmt(
                Query::custom_statement(&format!(
                    "DROP POLICY IF EXISTS operator_isolation_{table} ON {table};",
                    table = table_name
                ))
                .to_owned(),
            )
            .await?;
        manager
            .exec_stmt(
                Query::custom_statement(&format!(
                    "DROP POLICY IF EXISTS service_access_{table} ON {table};",
                    table = table_name
                ))
                .to_owned(),
            )
            .await?;

        // Disable RLS
        manager
            .exec_stmt(
                Query::custom_statement(&format!(
                    "ALTER TABLE {} DISABLE ROW LEVEL SECURITY;",
                    table_name
                ))
                .to_owned(),
            )
            .await?;

        Ok(())
    }
}
