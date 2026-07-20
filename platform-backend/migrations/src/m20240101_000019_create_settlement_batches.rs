use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SettlementBatch::Table)
                    .if_not_exists()
                    .col(pk_uuid(SettlementBatch::BatchId))
                    .col(uuid(SettlementBatch::OperatorId).not_null())
                    .col(string(SettlementBatch::ConnectorId).not_null())
                    .col(string(SettlementBatch::Status).not_null().default("pending"))
                    .col(big_integer(SettlementBatch::TotalAmountMinorUnits).not_null().default(0))
                    .col(string(SettlementBatch::Currency).not_null().default("AED"))
                    .col(integer(SettlementBatch::TotalRecords).not_null().default(0))
                    .col(integer(SettlementBatch::MatchedCount).not_null().default(0))
                    .col(integer(SettlementBatch::UnmatchedCount).not_null().default(0))
                    .col(integer(SettlementBatch::ExceptionCount).not_null().default(0))
                    .col(string(SettlementBatch::PeriodStart).not_null())
                    .col(string(SettlementBatch::PeriodEnd).not_null())
                    .col(json(SettlementBatch::Exceptions).null())
                    .col(timestamp_with_time_zone(SettlementBatch::PolledAt).null())
                    .col(timestamp_with_time_zone(SettlementBatch::MatchedAt).null())
                    .col(timestamp_with_time_zone(SettlementBatch::SettledAt).null())
                    .col(timestamp_with_time_zone(SettlementBatch::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(SettlementBatch::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_settlement_batch_operator")
                    .table(SettlementBatch::Table)
                    .col(SettlementBatch::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_settlement_batch_status")
                    .table(SettlementBatch::Table)
                    .col(SettlementBatch::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_settlement_batch_period")
                    .table(SettlementBatch::Table)
                    .col(SettlementBatch::PeriodStart)
                    .col(SettlementBatch::PeriodEnd)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SettlementBatch::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum SettlementBatch {
    Table,
    BatchId,
    OperatorId,
    ConnectorId,
    Status,
    TotalAmountMinorUnits,
    Currency,
    TotalRecords,
    MatchedCount,
    UnmatchedCount,
    ExceptionCount,
    PeriodStart,
    PeriodEnd,
    Exceptions,
    PolledAt,
    MatchedAt,
    SettledAt,
    CreatedAt,
    UpdatedAt,
}
