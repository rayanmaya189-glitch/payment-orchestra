//! Migration M003a — Reconciliation Tables
//!
//! Creates reconciliation-related tables:
//! - settlement_batches (BC-09)
//! - settlement_expectations (BC-09)
//! - fee_variances (BC-09)
//! - ledger_entries (BC-09)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_settlement_batches(manager).await?;
        self.create_settlement_expectations(manager).await?;
        self.create_fee_variances(manager).await?;
        self.create_ledger_entries(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(LedgerEntries::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(FeeVariances::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SettlementExpectations::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SettlementBatches::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
    async fn create_settlement_batches(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SettlementBatches::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SettlementBatches::SettlementBatchId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SettlementBatches::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(SettlementBatches::AcquirerLinkId).uuid().not_null())
                    .col(ColumnDef::new(SettlementBatches::BatchFileName).string_len(255).not_null())
                    .col(ColumnDef::new(SettlementBatches::FileChecksum).string_len(64).not_null())
                    .col(ColumnDef::new(SettlementBatches::FileFormat).string_len(32).not_null())
                    .col(ColumnDef::new(SettlementBatches::Status).string_len(32).not_null())
                    .col(ColumnDef::new(SettlementBatches::TotalTransactions).integer().not_null().default(0))
                    .col(ColumnDef::new(SettlementBatches::TotalAmountMinor).big_integer().not_null().default(0))
                    .col(ColumnDef::new(SettlementBatches::MatchedCount).integer().not_null().default(0))
                    .col(ColumnDef::new(SettlementBatches::UnmatchedCount).integer().not_null().default(0))
                    .col(ColumnDef::new(SettlementBatches::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(SettlementBatches::Records).json().not_null().default("[]"))
                    .col(ColumnDef::new(SettlementBatches::IngestedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(SettlementBatches::ProcessedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_sb_operator").table(SettlementBatches::Table).col(SettlementBatches::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_sb_checksum").table(SettlementBatches::Table).col(SettlementBatches::FileChecksum).unique().to_owned()).await?;
        manager.create_index(Index::create().name("idx_sb_status").table(SettlementBatches::Table).col(SettlementBatches::Status).to_owned()).await
    }

    async fn create_settlement_expectations(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SettlementExpectations::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SettlementExpectations::ExpectationId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SettlementExpectations::PaymentIntentId).uuid().not_null())
                    .col(ColumnDef::new(SettlementExpectations::AcquirerLinkId).uuid().not_null())
                    .col(ColumnDef::new(SettlementExpectations::ExpectedAmountMinor).big_integer().not_null())
                    .col(ColumnDef::new(SettlementExpectations::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(SettlementExpectations::ExpectedSettlementDate).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(SettlementExpectations::SettlementCycle).string_len(32).not_null())
                    .col(ColumnDef::new(SettlementExpectations::Status).string_len(32).not_null())
                    .col(ColumnDef::new(SettlementExpectations::SettledAmountMinor).big_integer().null())
                    .col(ColumnDef::new(SettlementExpectations::SettledAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(SettlementExpectations::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_se_status").table(SettlementExpectations::Table).col(SettlementExpectations::Status).to_owned()).await?;
        manager.create_index(Index::create().name("idx_se_pi").table(SettlementExpectations::Table).col(SettlementExpectations::PaymentIntentId).to_owned()).await
    }

    async fn create_fee_variances(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FeeVariances::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(FeeVariances::VarianceId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(FeeVariances::PaymentIntentId).uuid().not_null())
                    .col(ColumnDef::new(FeeVariances::AcquirerLinkId).uuid().not_null())
                    .col(ColumnDef::new(FeeVariances::ExpectedFeeMinor).big_integer().not_null())
                    .col(ColumnDef::new(FeeVariances::ActualFeeMinor).big_integer().not_null())
                    .col(ColumnDef::new(FeeVariances::VarianceAmountMinor).big_integer().not_null())
                    .col(ColumnDef::new(FeeVariances::VariancePercent).double().not_null())
                    .col(ColumnDef::new(FeeVariances::IsWithinTolerance).boolean().not_null().default(true))
                    .col(ColumnDef::new(FeeVariances::ToleranceThresholdPercent).double().not_null().default(5.0))
                    .col(ColumnDef::new(FeeVariances::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(FeeVariances::Status).string_len(32).not_null())
                    .col(ColumnDef::new(FeeVariances::Reason).text().null())
                    .col(ColumnDef::new(FeeVariances::DetectedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(FeeVariances::ResolvedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(FeeVariances::ResolutionNote).text().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_fv_status").table(FeeVariances::Table).col(FeeVariances::Status).to_owned()).await?;
        manager.create_index(Index::create().name("idx_fv_pi").table(FeeVariances::Table).col(FeeVariances::PaymentIntentId).to_owned()).await
    }

    async fn create_ledger_entries(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LedgerEntries::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LedgerEntries::EntryId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(LedgerEntries::TransactionId).uuid().not_null())
                    .col(ColumnDef::new(LedgerEntries::EntryType).string_len(16).not_null())
                    .col(ColumnDef::new(LedgerEntries::AmountMinor).big_integer().not_null())
                    .col(ColumnDef::new(LedgerEntries::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(LedgerEntries::SourceAcquirer).string_len(64).not_null())
                    .col(ColumnDef::new(LedgerEntries::ReconciliationBatchId).uuid().null())
                    .col(ColumnDef::new(LedgerEntries::Reconciled).boolean().not_null().default(false))
                    .col(ColumnDef::new(LedgerEntries::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_le_txn").table(LedgerEntries::Table).col(LedgerEntries::TransactionId).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum SettlementBatches {
    Table,
    SettlementBatchId,
    OperatorId,
    AcquirerLinkId,
    BatchFileName,
    FileChecksum,
    FileFormat,
    Status,
    TotalTransactions,
    TotalAmountMinor,
    MatchedCount,
    UnmatchedCount,
    Currency,
    Records,
    IngestedAt,
    ProcessedAt,
}

#[derive(DeriveIden)]
enum SettlementExpectations {
    Table,
    ExpectationId,
    PaymentIntentId,
    AcquirerLinkId,
    ExpectedAmountMinor,
    Currency,
    ExpectedSettlementDate,
    SettlementCycle,
    Status,
    SettledAmountMinor,
    SettledAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum FeeVariances {
    Table,
    VarianceId,
    PaymentIntentId,
    AcquirerLinkId,
    ExpectedFeeMinor,
    ActualFeeMinor,
    VarianceAmountMinor,
    VariancePercent,
    IsWithinTolerance,
    ToleranceThresholdPercent,
    Currency,
    Status,
    Reason,
    DetectedAt,
    ResolvedAt,
    ResolutionNote,
}

#[derive(DeriveIden)]
enum LedgerEntries {
    Table,
    EntryId,
    TransactionId,
    EntryType,
    AmountMinor,
    Currency,
    SourceAcquirer,
    ReconciliationBatchId,
    Reconciled,
    CreatedAt,
}
