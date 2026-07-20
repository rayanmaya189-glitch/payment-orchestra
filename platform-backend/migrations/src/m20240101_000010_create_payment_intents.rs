use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentIntent::Table)
                    .if_not_exists()
                    .col(pk_uuid(PaymentIntent::PaymentIntentId))
                    .col(uuid(PaymentIntent::OperatorId).not_null())
                    .col(string(PaymentIntent::Status).not_null().default("created"))
                    .col(big_integer(PaymentIntent::RequestedAmountMinorUnits).not_null())
                    .col(big_integer(PaymentIntent::AuthorizedAmountMinorUnits).not_null().default(0))
                    .col(big_integer(PaymentIntent::CapturedAmountMinorUnits).not_null().default(0))
                    .col(big_integer(PaymentIntent::RefundedAmountMinorUnits).not_null().default(0))
                    .col(string(PaymentIntent::Currency).not_null().default("AED"))
                    .col(string(PaymentIntent::IdempotencyKey).not_null().unique_key())
                    .col(uuid(PaymentIntent::PaymentMethodTokenId).null())
                    .col(uuid(PaymentIntent::RoutingPolicyId).null())
                    .col(integer(PaymentIntent::DeploymentEpoch).not_null().default(1))
                    .col(string(PaymentIntent::Purpose).not_null().default("payment"))
                    .col(json(PaymentIntent::Metadata).null())
                    .col(uuid(PaymentIntent::GatewayProfileId).null())
                    .col(integer(PaymentIntent::GatewayProfileVersion).null())
                    .col(string(PaymentIntent::GatewayRotationStrategy).null())
                    .col(string(PaymentIntent::GatewaySelectionReason).null())
                    .col(timestamp_with_time_zone(PaymentIntent::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(PaymentIntent::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_payment_intent_operator")
                    .table(PaymentIntent::Table)
                    .col(PaymentIntent::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_payment_intent_status")
                    .table(PaymentIntent::Table)
                    .col(PaymentIntent::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_payment_intent_created_at")
                    .table(PaymentIntent::Table)
                    .col(PaymentIntent::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaymentIntent::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum PaymentIntent {
    Table,
    PaymentIntentId,
    OperatorId,
    Status,
    RequestedAmountMinorUnits,
    AuthorizedAmountMinorUnits,
    CapturedAmountMinorUnits,
    RefundedAmountMinorUnits,
    Currency,
    IdempotencyKey,
    PaymentMethodTokenId,
    RoutingPolicyId,
    DeploymentEpoch,
    Purpose,
    Metadata,
    GatewayProfileId,
    GatewayProfileVersion,
    GatewayRotationStrategy,
    GatewaySelectionReason,
    CreatedAt,
    UpdatedAt,
}
