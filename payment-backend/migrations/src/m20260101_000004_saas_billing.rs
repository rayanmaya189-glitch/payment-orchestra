//! Migration M004 — SaaS Billing Tables
//!
//! Creates SaaS billing infrastructure for multi-tenant subscription management:
//! - saas_plans (subscription plans for merchants)
//! - tenant_subscriptions (merchant subscriptions to SaaS plans)
//! - tenant_usage (monthly usage tracking per tenant)
//! - saas_invoices (invoices for merchant billing)
//! - tenant_team_members (team member invitations)
//! - audit_logs (audit trail for compliance)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_saas_plans(manager).await?;
        self.create_tenant_subscriptions(manager).await?;
        self.create_tenant_usage(manager).await?;
        self.create_saas_invoices(manager).await?;
        self.create_tenant_team_members(manager).await?;
        self.create_audit_logs(manager).await?;
        self.seed_saas_plans(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(AuditLogs::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(TenantTeamMembers::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SaasInvoices::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(TenantUsage::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(TenantSubscriptions::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SaasPlans::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
    async fn create_saas_plans(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SaasPlans::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SaasPlans::PlanId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SaasPlans::Name).string_len(100).not_null())
                    .col(ColumnDef::new(SaasPlans::Slug).string_len(50).not_null())
                    .col(ColumnDef::new(SaasPlans::Description).text().null())
                    .col(ColumnDef::new(SaasPlans::PriceMonthlyMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(SaasPlans::PricePerTxnMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(SaasPlans::IncludedTxnsMonthly).integer().not_null().default(0))
                    .col(ColumnDef::new(SaasPlans::MaxGateways).integer().not_null().default(1))
                    .col(ColumnDef::new(SaasPlans::MaxTeamMembers).integer().not_null().default(1))
                    .col(ColumnDef::new(SaasPlans::MaxApiKeys).integer().not_null().default(1))
                    .col(ColumnDef::new(SaasPlans::MaxWebhooks).integer().not_null().default(1))
                    .col(ColumnDef::new(SaasPlans::DataRetentionDays).integer().not_null().default(30))
                    .col(ColumnDef::new(SaasPlans::Features).json().not_null().default("{}"))
                    .col(ColumnDef::new(SaasPlans::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(SaasPlans::SortOrder).integer().not_null().default(0))
                    .col(ColumnDef::new(SaasPlans::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(SaasPlans::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_saas_plans_slug").table(SaasPlans::Table).col(SaasPlans::Slug).unique().to_owned()).await
    }

    async fn create_tenant_subscriptions(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TenantSubscriptions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TenantSubscriptions::SubscriptionId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(TenantSubscriptions::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(TenantSubscriptions::PlanId).uuid().not_null())
                    .col(ColumnDef::new(TenantSubscriptions::Status).string_len(20).not_null())
                    .col(ColumnDef::new(TenantSubscriptions::CurrentPeriodStart).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(TenantSubscriptions::CurrentPeriodEnd).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(TenantSubscriptions::TrialEndsAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(TenantSubscriptions::CanceledAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(TenantSubscriptions::CancelReason).text().null())
                    .col(ColumnDef::new(TenantSubscriptions::PaymentMethodId).string_len(255).null())
                    .col(ColumnDef::new(TenantSubscriptions::StripeSubscriptionId).string_len(255).null())
                    .col(ColumnDef::new(TenantSubscriptions::CreatedBy).uuid().not_null())
                    .col(ColumnDef::new(TenantSubscriptions::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(TenantSubscriptions::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_ts_operator").table(TenantSubscriptions::Table).col(TenantSubscriptions::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_ts_status").table(TenantSubscriptions::Table).col(TenantSubscriptions::Status).to_owned()).await?;
        manager.create_index(Index::create().name("idx_ts_plan").table(TenantSubscriptions::Table).col(TenantSubscriptions::PlanId).to_owned()).await
    }

    async fn create_tenant_usage(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TenantUsage::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TenantUsage::UsageId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(TenantUsage::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(TenantUsage::PeriodStart).date().not_null())
                    .col(ColumnDef::new(TenantUsage::PeriodEnd).date().not_null())
                    .col(ColumnDef::new(TenantUsage::TransactionCount).integer().not_null().default(0))
                    .col(ColumnDef::new(TenantUsage::TransactionVolumeMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(TenantUsage::ApiCalls).integer().not_null().default(0))
                    .col(ColumnDef::new(TenantUsage::StorageBytes).big_integer().not_null().default(0))
                    .col(ColumnDef::new(TenantUsage::AiQueries).integer().not_null().default(0))
                    .col(ColumnDef::new(TenantUsage::OverageAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(TenantUsage::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(TenantUsage::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_tu_operator_period").table(TenantUsage::Table).col(TenantUsage::OperatorId).col(TenantUsage::PeriodStart).unique().to_owned()).await
    }

    async fn create_saas_invoices(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SaasInvoices::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SaasInvoices::InvoiceId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SaasInvoices::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(SaasInvoices::SubscriptionId).uuid().not_null())
                    .col(ColumnDef::new(SaasInvoices::InvoiceNumber).string_len(50).not_null())
                    .col(ColumnDef::new(SaasInvoices::Status).string_len(20).not_null())
                    .col(ColumnDef::new(SaasInvoices::SubtotalMinor).big_integer().not_null())
                    .col(ColumnDef::new(SaasInvoices::TaxMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(SaasInvoices::TotalMinor).big_integer().not_null())
                    .col(ColumnDef::new(SaasInvoices::Currency).string_len(3).not_null().default("USD"))
                    .col(ColumnDef::new(SaasInvoices::PeriodStart).date().not_null())
                    .col(ColumnDef::new(SaasInvoices::PeriodEnd).date().not_null())
                    .col(ColumnDef::new(SaasInvoices::DueDate).date().not_null())
                    .col(ColumnDef::new(SaasInvoices::PaidAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(SaasInvoices::LineItems).json().not_null().default("[]"))
                    .col(ColumnDef::new(SaasInvoices::StripeInvoiceId).string_len(255).null())
                    .col(ColumnDef::new(SaasInvoices::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(SaasInvoices::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_si_operator").table(SaasInvoices::Table).col(SaasInvoices::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_si_subscription").table(SaasInvoices::Table).col(SaasInvoices::SubscriptionId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_si_status").table(SaasInvoices::Table).col(SaasInvoices::Status).to_owned()).await
    }

    async fn create_tenant_team_members(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TenantTeamMembers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TenantTeamMembers::MembershipId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(TenantTeamMembers::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(TenantTeamMembers::PrincipalId).uuid().not_null())
                    .col(ColumnDef::new(TenantTeamMembers::Role).string_len(64).not_null())
                    .col(ColumnDef::new(TenantTeamMembers::InvitedBy).uuid().not_null())
                    .col(ColumnDef::new(TenantTeamMembers::InvitedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(TenantTeamMembers::AcceptedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(TenantTeamMembers::Status).string_len(20).not_null().default("pending"))
                    .col(ColumnDef::new(TenantTeamMembers::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(TenantTeamMembers::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_ttm_operator").table(TenantTeamMembers::Table).col(TenantTeamMembers::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_ttm_principal").table(TenantTeamMembers::Table).col(TenantTeamMembers::PrincipalId).to_owned()).await
    }

    async fn create_audit_logs(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditLogs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AuditLogs::LogId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(AuditLogs::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(AuditLogs::PrincipalId).uuid().not_null())
                    .col(ColumnDef::new(AuditLogs::Action).string_len(100).not_null())
                    .col(ColumnDef::new(AuditLogs::Resource).string_len(100).not_null())
                    .col(ColumnDef::new(AuditLogs::ResourceId).string_len(255).null())
                    .col(ColumnDef::new(AuditLogs::OldValue).json().null())
                    .col(ColumnDef::new(AuditLogs::NewValue).json().null())
                    .col(ColumnDef::new(AuditLogs::IpAddress).string_len(45).null())
                    .col(ColumnDef::new(AuditLogs::UserAgent).string_len(512).null())
                    .col(ColumnDef::new(AuditLogs::Metadata).json().null())
                    .col(ColumnDef::new(AuditLogs::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_al_operator").table(AuditLogs::Table).col(AuditLogs::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_al_created").table(AuditLogs::Table).col(AuditLogs::CreatedAt).to_owned()).await?;
        manager.create_index(Index::create().name("idx_al_action").table(AuditLogs::Table).col(AuditLogs::Action).to_owned()).await
    }

    async fn seed_saas_plans(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        use sea_orm_migration::prelude::*;

        let now = chrono::Utc::now();
        
        // Free plan
        manager.exec_stmt(
            Query::insert()
                .into_table(SaasPlans::Table)
                .columns(vec![
                    SaasPlans::PlanId,
                    SaasPlans::Name,
                    SaasPlans::Slug,
                    SaasPlans::Description,
                    SaasPlans::PriceMonthlyMinor,
                    SaasPlans::PricePerTxnMinor,
                    SaasPlans::IncludedTxnsMonthly,
                    SaasPlans::MaxGateways,
                    SaasPlans::MaxTeamMembers,
                    SaasPlans::MaxApiKeys,
                    SaasPlans::MaxWebhooks,
                    SaasPlans::DataRetentionDays,
                    SaasPlans::Features,
                    SaasPlans::IsActive,
                    SaasPlans::SortOrder,
                    SaasPlans::CreatedAt,
                    SaasPlans::UpdatedAt,
                ])
                .values_panic(vec![
                    Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap().into(),
                    "Free".into(),
                    "free".into(),
                    "Get started with sandbox mode and basic features".into(),
                    0_i64.into(),
                    30_i64.into(),
                    100_i32.into(),
                    1_i32.into(),
                    1_i32.into(),
                    1_i32.into(),
                    1_i32.into(),
                    30_i32.into(),
                    serde_json::json!({"sandbox_only": true, "email_support": false}).to_string().into(),
                    true.into(),
                    0_i32.into(),
                    now.into(),
                    now.into(),
                ])
                .to_owned(),
        ).await?;

        // Starter plan
        manager.exec_stmt(
            Query::insert()
                .into_table(SaasPlans::Table)
                .columns(vec![
                    SaasPlans::PlanId,
                    SaasPlans::Name,
                    SaasPlans::Slug,
                    SaasPlans::Description,
                    SaasPlans::PriceMonthlyMinor,
                    SaasPlans::PricePerTxnMinor,
                    SaasPlans::IncludedTxnsMonthly,
                    SaasPlans::MaxGateways,
                    SaasPlans::MaxTeamMembers,
                    SaasPlans::MaxApiKeys,
                    SaasPlans::MaxWebhooks,
                    SaasPlans::DataRetentionDays,
                    SaasPlans::Features,
                    SaasPlans::IsActive,
                    SaasPlans::SortOrder,
                    SaasPlans::CreatedAt,
                    SaasPlans::UpdatedAt,
                ])
                .values_panic(vec![
                    Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap().into(),
                    "Starter".into(),
                    "starter".into(),
                    "Perfect for small businesses getting started with payment orchestration".into(),
                    9900_i64.into(),
                    15_i64.into(),
                    1000_i32.into(),
                    3_i32.into(),
                    5_i32.into(),
                    3_i32.into(),
                    5_i32.into(),
                    365_i32.into(),
                    serde_json::json!({"analytics": true, "webhooks": true, "email_support": true}).to_string().into(),
                    true.into(),
                    1_i32.into(),
                    now.into(),
                    now.into(),
                ])
                .to_owned(),
        ).await?;

        // Professional plan
        manager.exec_stmt(
            Query::insert()
                .into_table(SaasPlans::Table)
                .columns(vec![
                    SaasPlans::PlanId,
                    SaasPlans::Name,
                    SaasPlans::Slug,
                    SaasPlans::Description,
                    SaasPlans::PriceMonthlyMinor,
                    SaasPlans::PricePerTxnMinor,
                    SaasPlans::IncludedTxnsMonthly,
                    SaasPlans::MaxGateways,
                    SaasPlans::MaxTeamMembers,
                    SaasPlans::MaxApiKeys,
                    SaasPlans::MaxWebhooks,
                    SaasPlans::DataRetentionDays,
                    SaasPlans::Features,
                    SaasPlans::IsActive,
                    SaasPlans::SortOrder,
                    SaasPlans::CreatedAt,
                    SaasPlans::UpdatedAt,
                ])
                .values_panic(vec![
                    Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap().into(),
                    "Professional".into(),
                    "professional".into(),
                    "For growing businesses that need advanced features and priority support".into(),
                    49900_i64.into(),
                    8_i64.into(),
                    10000_i32.into(),
                    (-1_i32).into(), // Unlimited
                    20_i32.into(),
                    10_i32.into(),
                    20_i32.into(),
                    730_i32.into(),
                    serde_json::json!({"analytics": true, "webhooks": true, "ai_assistant": true, "priority_support": true, "advanced_routing": true}).to_string().into(),
                    true.into(),
                    2_i32.into(),
                    now.into(),
                    now.into(),
                ])
                .to_owned(),
        ).await?;

        // Enterprise plan
        manager.exec_stmt(
            Query::insert()
                .into_table(SaasPlans::Table)
                .columns(vec![
                    SaasPlans::PlanId,
                    SaasPlans::Name,
                    SaasPlans::Slug,
                    SaasPlans::Description,
                    SaasPlans::PriceMonthlyMinor,
                    SaasPlans::PricePerTxnMinor,
                    SaasPlans::IncludedTxnsMonthly,
                    SaasPlans::MaxGateways,
                    SaasPlans::MaxTeamMembers,
                    SaasPlans::MaxApiKeys,
                    SaasPlans::MaxWebhooks,
                    SaasPlans::DataRetentionDays,
                    SaasPlans::Features,
                    SaasPlans::IsActive,
                    SaasPlans::SortOrder,
                    SaasPlans::CreatedAt,
                    SaasPlans::UpdatedAt,
                ])
                .values_panic(vec![
                    Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap().into(),
                    "Enterprise".into(),
                    "enterprise".into(),
                    "Custom pricing for large enterprises with dedicated support and SLA".into(),
                    0_i64.into(),
                    0_i64.into(),
                    (-1_i32).into(), // Unlimited
                    (-1_i32).into(), // Unlimited
                    (-1_i32).into(), // Unlimited
                    (-1_i32).into(), // Unlimited
                    (-1_i32).into(), // Unlimited
                    (-1_i32).into(), // Unlimited
                    serde_json::json!({"analytics": true, "webhooks": true, "ai_assistant": true, "priority_support": true, "advanced_routing": true, "custom_branding": true, "sso": true, "dedicated_support": true, "sla": true}).to_string().into(),
                    true.into(),
                    3_i32.into(),
                    now.into(),
                    now.into(),
                ])
                .to_owned(),
        ).await?;

        Ok(())
    }
}

// ─── Table Names ─────────────────────────────────────────────────────────────

#[derive(DeriveIden)]
enum SaasPlans {
    Table,
    PlanId,
    Name,
    Slug,
    Description,
    PriceMonthlyMinor,
    PricePerTxnMinor,
    IncludedTxnsMonthly,
    MaxGateways,
    MaxTeamMembers,
    MaxApiKeys,
    MaxWebhooks,
    DataRetentionDays,
    Features,
    IsActive,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum TenantSubscriptions {
    Table,
    SubscriptionId,
    OperatorId,
    PlanId,
    Status,
    CurrentPeriodStart,
    CurrentPeriodEnd,
    TrialEndsAt,
    CanceledAt,
    CancelReason,
    PaymentMethodId,
    StripeSubscriptionId,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum TenantUsage {
    Table,
    UsageId,
    OperatorId,
    PeriodStart,
    PeriodEnd,
    TransactionCount,
    TransactionVolumeMinor,
    ApiCalls,
    StorageBytes,
    AiQueries,
    OverageAmountMinor,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SaasInvoices {
    Table,
    InvoiceId,
    OperatorId,
    SubscriptionId,
    InvoiceNumber,
    Status,
    SubtotalMinor,
    TaxMinor,
    TotalMinor,
    Currency,
    PeriodStart,
    PeriodEnd,
    DueDate,
    PaidAt,
    LineItems,
    StripeInvoiceId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum TenantTeamMembers {
    Table,
    MembershipId,
    OperatorId,
    PrincipalId,
    Role,
    InvitedBy,
    InvitedAt,
    AcceptedAt,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AuditLogs {
    Table,
    LogId,
    OperatorId,
    PrincipalId,
    Action,
    Resource,
    ResourceId,
    OldValue,
    NewValue,
    IpAddress,
    UserAgent,
    Metadata,
    CreatedAt,
}
