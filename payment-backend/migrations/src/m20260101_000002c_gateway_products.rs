//! Migration M002c — Gateway & Product Tables
//!
//! Creates gateway and product-related tables:
//! - gateway_profiles (BC-04)
//! - merchant_acquirer_links (BC-04)
//! - chargeback_cases (BC-10)
//! - invoices (BC-07)
//! - subscriptions (BC-08)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_gateway_profiles(manager).await?;
        self.create_merchant_acquirer_links(manager).await?;
        self.create_chargeback_cases(manager).await?;
        self.create_invoices(manager).await?;
        self.create_subscriptions(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Subscriptions::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Invoices::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ChargebackCases::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(MerchantAcquirerLinks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(GatewayProfiles::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
    async fn create_gateway_profiles(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GatewayProfiles::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(GatewayProfiles::ProfileId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(GatewayProfiles::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(GatewayProfiles::ConnectorId).string_len(64).not_null())
                    .col(ColumnDef::new(GatewayProfiles::MerchantAcquirerLinkId).uuid().not_null())
                    .col(ColumnDef::new(GatewayProfiles::Status).string_len(16).not_null())
                    .col(ColumnDef::new(GatewayProfiles::LimitsMinAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::LimitsMaxAmountMinor).big_integer().not_null())
                    .col(ColumnDef::new(GatewayProfiles::LimitsDailyVolumeMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::LimitsMonthlyVolumeMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::LimitsMaxRefundMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::FeeFixedMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::FeePercentageBps).integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::FeeCrossBorderBps).integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::FeeCurrencyConversionBps).integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::FeeMaxCap).big_integer().null())
                    .col(ColumnDef::new(GatewayProfiles::FeeMinFloor).big_integer().null())
                    .col(ColumnDef::new(GatewayProfiles::RoutingPriority).integer().not_null().default(0))
                    .col(ColumnDef::new(GatewayProfiles::EnabledCardSchemes).json().not_null().default("[]"))
                    .col(ColumnDef::new(GatewayProfiles::EnabledCurrencies).json().not_null().default("[]"))
                    .col(ColumnDef::new(GatewayProfiles::EnabledCountries).json().not_null().default("[]"))
                    .col(ColumnDef::new(GatewayProfiles::RateLimitPerSecond).integer().not_null().default(100))
                    .col(ColumnDef::new(GatewayProfiles::RateLimitPerDay).integer().not_null().default(100000))
                    .col(ColumnDef::new(GatewayProfiles::RateLimitBurst).integer().not_null().default(200))
                    .col(ColumnDef::new(GatewayProfiles::MonitoringSuccessRateAlert).double().not_null().default(0.95))
                    .col(ColumnDef::new(GatewayProfiles::MonitoringSuccessRateCritical).double().not_null().default(0.90))
                    .col(ColumnDef::new(GatewayProfiles::MonitoringLatencyP99AlertMs).integer().not_null().default(3000))
                    .col(ColumnDef::new(GatewayProfiles::MonitoringLatencyP99CriticalMs).integer().not_null().default(5000))
                    .col(ColumnDef::new(GatewayProfiles::MonitoringAutoDisable).boolean().not_null().default(false))
                    .col(ColumnDef::new(GatewayProfiles::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(GatewayProfiles::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_gp_operator").table(GatewayProfiles::Table).col(GatewayProfiles::OperatorId).to_owned()).await
    }

    async fn create_merchant_acquirer_links(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MerchantAcquirerLinks::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(MerchantAcquirerLinks::LinkId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(MerchantAcquirerLinks::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::ConnectorId).string_len(64).not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::DisplayName).string_len(128).not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::Environment).string_len(16).not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::EncryptedCredentials).binary().not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::CredentialsHash).string_len(64).not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::Status).string_len(16).not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::HealthStatus).string_len(16).not_null().default("unknown"))
                    .col(ColumnDef::new(MerchantAcquirerLinks::LastTestedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::LastHealthyAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::CredentialsExpiresAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(MerchantAcquirerLinks::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_mal_operator").table(MerchantAcquirerLinks::Table).col(MerchantAcquirerLinks::OperatorId).to_owned()).await
    }

    async fn create_chargeback_cases(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChargebackCases::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ChargebackCases::ChargebackId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ChargebackCases::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(ChargebackCases::PaymentIntentId).uuid().not_null())
                    .col(ColumnDef::new(ChargebackCases::AcquirerLinkId).uuid().not_null())
                    .col(ColumnDef::new(ChargebackCases::Status).string_len(32).not_null())
                    .col(ColumnDef::new(ChargebackCases::ReasonCode).string_len(64).not_null())
                    .col(ColumnDef::new(ChargebackCases::AmountMinorUnits).big_integer().not_null())
                    .col(ColumnDef::new(ChargebackCases::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(ChargebackCases::ReceivedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(ChargebackCases::RepresentmentDeadline).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(ChargebackCases::ResolvedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(ChargebackCases::Outcome).string_len(32).null())
                    .col(ColumnDef::new(ChargebackCases::ResolutionNote).text().null())
                    .col(ColumnDef::new(ChargebackCases::Submissions).json().not_null().default("[]"))
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_cb_pi").table(ChargebackCases::Table).col(ChargebackCases::PaymentIntentId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_cb_status").table(ChargebackCases::Table).col(ChargebackCases::Status).to_owned()).await
    }

    async fn create_invoices(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Invoices::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Invoices::InvoiceId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Invoices::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(Invoices::OrderReference).string_len(255).not_null())
                    .col(ColumnDef::new(Invoices::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Invoices::LineItems).json().not_null().default("[]"))
                    .col(ColumnDef::new(Invoices::TotalAmountMinor).big_integer().not_null())
                    .col(ColumnDef::new(Invoices::PaidAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Invoices::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(Invoices::DueDate).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Invoices::RecipientEmail).string_len(255).null())
                    .col(ColumnDef::new(Invoices::PaymentIntentIds).json().not_null().default("[]"))
                    .col(ColumnDef::new(Invoices::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Invoices::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_inv_operator").table(Invoices::Table).col(Invoices::OperatorId).to_owned()).await
    }

    async fn create_subscriptions(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Subscriptions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Subscriptions::SubscriptionId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Subscriptions::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(Subscriptions::CustomerId).uuid().not_null())
                    .col(ColumnDef::new(Subscriptions::PlanId).string_len(128).not_null())
                    .col(ColumnDef::new(Subscriptions::PlanAmountMinorUnits).big_integer().not_null())
                    .col(ColumnDef::new(Subscriptions::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(Subscriptions::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Subscriptions::CurrentPeriodStart).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Subscriptions::CurrentPeriodEnd).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Subscriptions::BillingIntervalDays).big_integer().not_null())
                    .col(ColumnDef::new(Subscriptions::PaymentMethodTokenId).uuid().null())
                    .col(ColumnDef::new(Subscriptions::DunningRetryCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Subscriptions::MaxDunningRetries).integer().not_null().default(3))
                    .col(ColumnDef::new(Subscriptions::BillingCycles).json().not_null().default("[]"))
                    .col(ColumnDef::new(Subscriptions::DunningRetries).json().not_null().default("[]"))
                    .col(ColumnDef::new(Subscriptions::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Subscriptions::CancelledAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Subscriptions::PausedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Subscriptions::ResumedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_sub_operator").table(Subscriptions::Table).col(Subscriptions::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_sub_status").table(Subscriptions::Table).col(Subscriptions::Status).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum GatewayProfiles {
    Table,
    ProfileId,
    OperatorId,
    ConnectorId,
    MerchantAcquirerLinkId,
    Status,
    LimitsMinAmountMinor,
    LimitsMaxAmountMinor,
    LimitsDailyVolumeMinor,
    LimitsMonthlyVolumeMinor,
    LimitsMaxRefundMinor,
    FeeFixedMinor,
    FeePercentageBps,
    FeeCrossBorderBps,
    FeeCurrencyConversionBps,
    FeeMaxCap,
    FeeMinFloor,
    RoutingPriority,
    EnabledCardSchemes,
    EnabledCurrencies,
    EnabledCountries,
    RateLimitPerSecond,
    RateLimitPerDay,
    RateLimitBurst,
    MonitoringSuccessRateAlert,
    MonitoringSuccessRateCritical,
    MonitoringLatencyP99AlertMs,
    MonitoringLatencyP99CriticalMs,
    MonitoringAutoDisable,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MerchantAcquirerLinks {
    Table,
    LinkId,
    OperatorId,
    ConnectorId,
    DisplayName,
    Environment,
    EncryptedCredentials,
    CredentialsHash,
    Status,
    HealthStatus,
    LastTestedAt,
    LastHealthyAt,
    CredentialsExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ChargebackCases {
    Table,
    ChargebackId,
    OperatorId,
    PaymentIntentId,
    AcquirerLinkId,
    Status,
    ReasonCode,
    AmountMinorUnits,
    Currency,
    ReceivedAt,
    RepresentmentDeadline,
    ResolvedAt,
    Outcome,
    ResolutionNote,
    Submissions,
}

#[derive(DeriveIden)]
enum Invoices {
    Table,
    InvoiceId,
    OperatorId,
    OrderReference,
    Status,
    LineItems,
    TotalAmountMinor,
    PaidAmountMinor,
    Currency,
    DueDate,
    RecipientEmail,
    PaymentIntentIds,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Subscriptions {
    Table,
    SubscriptionId,
    OperatorId,
    CustomerId,
    PlanId,
    PlanAmountMinorUnits,
    Currency,
    Status,
    CurrentPeriodStart,
    CurrentPeriodEnd,
    BillingIntervalDays,
    PaymentMethodTokenId,
    DunningRetryCount,
    MaxDunningRetries,
    BillingCycles,
    DunningRetries,
    CreatedAt,
    CancelledAt,
    PausedAt,
    ResumedAt,
}
