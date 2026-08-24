//! Migration M002a — Payment Core Tables
//!
//! Creates payment core tables:
//! - payment_intents (AGG-01)
//! - routing_policies (AGG-02)
//! - payment_method_tokens (AGG-03)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_payment_intents(manager).await?;
        self.create_routing_policies(manager).await?;
        self.create_payment_method_tokens(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PaymentMethodTokens::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(RoutingPolicies::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PaymentIntents::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
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
}

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
