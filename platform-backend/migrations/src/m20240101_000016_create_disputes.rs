use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Dispute::Table)
                    .if_not_exists()
                    .col(pk_uuid(Dispute::DisputeId))
                    .col(uuid(Dispute::PaymentIntentId).not_null())
                    .col(uuid(Dispute::OperatorId).not_null())
                    .col(string(Dispute::Status).not_null().default("opened"))
                    .col(string(Dispute::Reason).not_null())
                    .col(string(Dispute::ReasonCode).null())
                    .col(big_integer(Dispute::DisputedAmountMinorUnits).not_null())
                    .col(string(Dispute::Currency).not_null().default("AED"))
                    .col(string(Dispute::AcquirerReference).not_null())
                    .col(string(Dispute::ConnectorId).not_null())
                    .col(string(Dispute::AcquirerDisputeId).null())
                    .col(json(Dispute::Evidence).null())
                    .col(string(Dispute::Decision).null())
                    .col(string(Dispute::DecisionReason).null())
                    .col(timestamp_with_time_zone(Dispute::OpenedAt).not_null())
                    .col(timestamp_with_time_zone(Dispute::RespondBy).null())
                    .col(timestamp_with_time_zone(Dispute::ResolvedAt).null())
                    .col(timestamp_with_time_zone(Dispute::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Dispute::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispute_payment_intent")
                    .table(Dispute::Table)
                    .col(Dispute::PaymentIntentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispute_operator")
                    .table(Dispute::Table)
                    .col(Dispute::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispute_status")
                    .table(Dispute::Table)
                    .col(Dispute::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Dispute::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Dispute {
    Table,
    DisputeId,
    PaymentIntentId,
    OperatorId,
    Status,
    Reason,
    ReasonCode,
    DisputedAmountMinorUnits,
    Currency,
    AcquirerReference,
    ConnectorId,
    AcquirerDisputeId,
    Evidence,
    Decision,
    DecisionReason,
    OpenedAt,
    RespondBy,
    ResolvedAt,
    CreatedAt,
    UpdatedAt,
}
