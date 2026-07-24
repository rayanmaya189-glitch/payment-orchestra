//! Migration M002 — Core Business Domain Tables
//!
//! Creates tables for the primary business services:
//! - payment_intents, routing_policies, payment_method_tokens (BC-05)
//! - operators (BC-02)
//! - principals, api_keys, role_assignments (BC-01)
//! - gateway_profiles, merchant_acquirer_links (BC-04)
//! - chargeback_cases (BC-10)
//! - invoices (BC-07)
//! - subscriptions (BC-08)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_payment_intents(manager).await?;
        self.create_routing_policies(manager).await?;
        self.create_payment_method_tokens(manager).await?;
        self.create_operators(manager).await?;
        self.create_principals(manager).await?;
        self.create_api_keys(manager).await?;
        self.create_role_assignments(manager).await?;
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
        manager.drop_table(Table::drop().table(RoleAssignments::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ApiKeys::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Principals::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Operators::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PaymentMethodTokens::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(RoutingPolicies::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PaymentIntents::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
    // ── AGG-01: PaymentIntent (BC-05) ───────────────────────────────────
    async fn create_payment_intents(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentIntents::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PaymentIntents::PaymentIntentId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(PaymentIntents::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(PaymentIntents::AmountMinorUnits).big_integer().not_null())
                    .col(ColumnDef::new(PaymentIntents::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(PaymentIntents::Status).string_len(32).not_null())
                    .col(ColumnDef::new(PaymentIntents::AuthorizedAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(PaymentIntents::CapturedAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(PaymentIntents::RefundedAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(PaymentIntents::IdempotencyKey).string_len(255).not_null())
                    .col(ColumnDef::new(PaymentIntents::Purpose).string_len(32).not_null().default("payment"))
                    .col(ColumnDef::new(PaymentIntents::SourceType).string_len(32).null())
                    .col(ColumnDef::new(PaymentIntents::SourceId).uuid().null())
                    .col(ColumnDef::new(PaymentIntents::PaymentMethodTokenId).uuid().null())
                    .col(ColumnDef::new(PaymentIntents::RoutingPolicyId).uuid().null())
                    .col(ColumnDef::new(PaymentIntents::GatewayProfileId).uuid().null())
                    .col(ColumnDef::new(PaymentIntents::RiskScore).double().null())
                    .col(ColumnDef::new(PaymentIntents::RiskLevel).string_len(16).null())
                    .col(ColumnDef::new(PaymentIntents::MetadataJson).json().null())
                    .col(ColumnDef::new(PaymentIntents::RoutingAttempts).json().not_null().default("[]"))
                    .col(ColumnDef::new(PaymentIntents::Version).big_integer().not_null().default(0))
                    .col(ColumnDef::new(PaymentIntents::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(PaymentIntents::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_pi_operator").table(PaymentIntents::Table).col(PaymentIntents::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_pi_status").table(PaymentIntents::Table).col(PaymentIntents::Status).to_owned()).await?;
        manager.create_index(Index::create().name("idx_pi_idempotency").table(PaymentIntents::Table).col(PaymentIntents::IdempotencyKey).unique().to_owned()).await
    }

    // ── AGG-02: RoutingPolicy (BC-05) ───────────────────────────────────
    async fn create_routing_policies(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RoutingPolicies::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RoutingPolicies::RoutingPolicyId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(RoutingPolicies::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(RoutingPolicies::Version).integer().not_null().default(1))
                    .col(ColumnDef::new(RoutingPolicies::Status).string_len(16).not_null())
                    .col(ColumnDef::new(RoutingPolicies::Rules).json().not_null().default("[]"))
                    .col(ColumnDef::new(RoutingPolicies::FailoverConfig).json().null())
                    .col(ColumnDef::new(RoutingPolicies::RotationStrategy).string_len(32).not_null().default("priority"))
                    .col(ColumnDef::new(RoutingPolicies::PartialAuthStrategy).string_len(32).not_null().default("accept_partial"))
                    .col(ColumnDef::new(RoutingPolicies::MaxTransactionAmountMinor).big_integer().null())
                    .col(ColumnDef::new(RoutingPolicies::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(RoutingPolicies::ActivatedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_rp_operator_active").table(RoutingPolicies::Table).col(RoutingPolicies::OperatorId).col(RoutingPolicies::Status).to_owned()).await
    }

    // ── AGG-03: PaymentMethodToken (BC-05) ──────────────────────────────
    async fn create_payment_method_tokens(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentMethodTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PaymentMethodTokens::TokenId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(PaymentMethodTokens::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::PaymentMethodType).string_len(32).not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::LastFour).string_len(4).not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::CardBrand).string_len(32).null())
                    .col(ColumnDef::new(PaymentMethodTokens::ExpiryMonth).small_integer().null())
                    .col(ColumnDef::new(PaymentMethodTokens::ExpiryYear).small_integer().null())
                    .col(ColumnDef::new(PaymentMethodTokens::TokenStatus).string_len(16).not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::AcquirerLinkId).uuid().not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::AcquirerTokenReference).string_len(255).not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::EncryptedToken).binary().not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(PaymentMethodTokens::ExpiresAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(PaymentMethodTokens::RevokedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(PaymentMethodTokens::RevocationReason).text().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_pmt_operator").table(PaymentMethodTokens::Table).col(PaymentMethodTokens::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_pmt_status").table(PaymentMethodTokens::Table).col(PaymentMethodTokens::TokenStatus).to_owned()).await?;
        manager.create_index(Index::create().name("idx_pmt_acquirer_ref").table(PaymentMethodTokens::Table).col(PaymentMethodTokens::AcquirerTokenReference).unique().to_owned()).await
    }

    // ── BC-02: Operator ─────────────────────────────────────────────────
    async fn create_operators(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Operators::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Operators::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Operators::LegalName).string_len(255).not_null())
                    .col(ColumnDef::new(Operators::TradeLicenseNo).string_len(255).not_null())
                    .col(ColumnDef::new(Operators::Country).string_len(2).not_null().default("AE"))
                    .col(ColumnDef::new(Operators::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Operators::Subdomain).string_len(64).not_null())
                    .col(ColumnDef::new(Operators::Email).string_len(255).not_null())
                    .col(ColumnDef::new(Operators::VerificationTokenHash).string_len(255).null())
                    .col(ColumnDef::new(Operators::ProvisionedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Operators::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Operators::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_op_subdomain").table(Operators::Table).col(Operators::Subdomain).unique().to_owned()).await
    }

    // ── BC-01: Principal ────────────────────────────────────────────────
    async fn create_principals(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Principals::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Principals::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Principals::PrincipalType).string_len(32).not_null())
                    .col(ColumnDef::new(Principals::Email).string_len(255).null())
                    .col(ColumnDef::new(Principals::PasswordHash).binary().null())
                    .col(ColumnDef::new(Principals::MfaEnrolled).boolean().not_null().default(false))
                    .col(ColumnDef::new(Principals::MfaMethod).string_len(32).null())
                    .col(ColumnDef::new(Principals::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Principals::FailedLoginAttempts).integer().not_null().default(0))
                    .col(ColumnDef::new(Principals::LockedUntil).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Principals::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Principals::LastLoginAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Principals::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_principal_email").table(Principals::Table).col(Principals::Email).unique().to_owned()).await
    }

    // ── BC-01: ApiKey ───────────────────────────────────────────────────
    async fn create_api_keys(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKeys::ApiKeyId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiKeys::PrincipalId).uuid().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).string_len(128).not_null())
                    .col(ColumnDef::new(ApiKeys::KeyPrefix).string_len(8).not_null())
                    .col(ColumnDef::new(ApiKeys::KeyHash).binary().not_null())
                    .col(ColumnDef::new(ApiKeys::Scopes).json().not_null().default("[]"))
                    .col(ColumnDef::new(ApiKeys::Status).string_len(16).not_null())
                    .col(ColumnDef::new(ApiKeys::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(ApiKeys::ExpiresAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(ApiKeys::LastUsedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_ak_principal").table(ApiKeys::Table).col(ApiKeys::PrincipalId).to_owned()).await
    }

    // ── BC-01: RoleAssignment ───────────────────────────────────────────
    async fn create_role_assignments(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RoleAssignments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RoleAssignments::PrincipalId).uuid().not_null())
                    .col(ColumnDef::new(RoleAssignments::Role).string_len(64).not_null())
                    .col(ColumnDef::new(RoleAssignments::AbacConditions).json().null())
                    .primary_key(
                        Index::create()
                            .col(RoleAssignments::PrincipalId)
                            .col(RoleAssignments::Role),
                    )
                    .to_owned(),
            )
            .await
    }

    // ── BC-04: GatewayProfile ───────────────────────────────────────────
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

    // ── BC-04: MerchantAcquirerLink ─────────────────────────────────────
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

    // ── BC-10: ChargebackCase ───────────────────────────────────────────
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

    // ── BC-07: Invoice ──────────────────────────────────────────────────
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

    // ── BC-08: Subscription ─────────────────────────────────────────────
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

// ─── Column Identifiers ──────────────────────────────────────────────────────

#[derive(DeriveIden)]
enum PaymentIntents {
    Table,
    PaymentIntentId,
    OperatorId,
    AmountMinorUnits,
    Currency,
    Status,
    AuthorizedAmountMinor,
    CapturedAmountMinor,
    RefundedAmountMinor,
    IdempotencyKey,
    Purpose,
    SourceType,
    SourceId,
    PaymentMethodTokenId,
    RoutingPolicyId,
    GatewayProfileId,
    RiskScore,
    RiskLevel,
    MetadataJson,
    RoutingAttempts,
    Version,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RoutingPolicies {
    Table,
    RoutingPolicyId,
    OperatorId,
    Version,
    Status,
    Rules,
    FailoverConfig,
    RotationStrategy,
    PartialAuthStrategy,
    MaxTransactionAmountMinor,
    CreatedAt,
    ActivatedAt,
}

#[derive(DeriveIden)]
enum PaymentMethodTokens {
    Table,
    TokenId,
    OperatorId,
    PaymentMethodType,
    LastFour,
    CardBrand,
    ExpiryMonth,
    ExpiryYear,
    TokenStatus,
    AcquirerLinkId,
    AcquirerTokenReference,
    EncryptedToken,
    CreatedAt,
    ExpiresAt,
    RevokedAt,
    RevocationReason,
}

#[derive(DeriveIden)]
enum Operators {
    Table,
    Id,
    LegalName,
    TradeLicenseNo,
    Country,
    Status,
    Subdomain,
    Email,
    VerificationTokenHash,
    ProvisionedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Principals {
    Table,
    Id,
    PrincipalType,
    Email,
    PasswordHash,
    MfaEnrolled,
    MfaMethod,
    Status,
    FailedLoginAttempts,
    LockedUntil,
    CreatedAt,
    LastLoginAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    ApiKeyId,
    PrincipalId,
    Name,
    KeyPrefix,
    KeyHash,
    Scopes,
    Status,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum RoleAssignments {
    Table,
    PrincipalId,
    Role,
    AbacConditions,
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
