//! Migration M003 — Supporting Domain Tables
//!
//! Creates tables for supporting services:
//! - settlement_batches, settlement_expectations, fee_variances, ledger_entries (BC-09)
//! - kyb_cases, aml_alerts (BC-03)
//! - payment_links (BC-11)
//! - notifications (BC-14)
//! - documents (BC-13)
//! - risk_assessments (BC-12)
//! - saga_executions (saga-coordinator)
//! - scheduled_jobs (scheduler)
//! - onboarding_requests (merchant-connector-onboarding)
//! - pending_changes (iam-service)
//! - webhook_subscriptions, webhook_deliveries

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
        self.create_kyb_cases(manager).await?;
        self.create_aml_alerts(manager).await?;
        self.create_payment_links(manager).await?;
        self.create_notifications(manager).await?;
        self.create_documents(manager).await?;
        self.create_risk_assessments(manager).await?;
        self.create_saga_executions(manager).await?;
        self.create_scheduled_jobs(manager).await?;
        self.create_onboarding_requests(manager).await?;
        self.create_pending_changes(manager).await?;
        self.create_webhook_subscriptions(manager).await?;
        self.create_webhook_deliveries(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(WebhookDeliveries::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(WebhookSubscriptions::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PendingChanges::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(OnboardingRequests::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ScheduledJobs::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SagaExecutions::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(RiskAssessments::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Documents::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Notifications::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PaymentLinks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AmlAlerts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(KybCases::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(LedgerEntries::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(FeeVariances::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SettlementExpectations::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SettlementBatches::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
    // ── BC-09: SettlementBatch ──────────────────────────────────────────
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

    // ── BC-09: SettlementExpectation ─────────────────────────────────────
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

    // ── BC-09: FeeVariance ──────────────────────────────────────────────
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

    // ── BC-09: LedgerEntry ──────────────────────────────────────────────
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

    // ── BC-03: KybCase ──────────────────────────────────────────────────
    async fn create_kyb_cases(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KybCases::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KybCases::KybCaseId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(KybCases::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(KybCases::Status).string_len(32).not_null())
                    .col(ColumnDef::new(KybCases::DocumentIds).json().not_null().default("[]"))
                    .col(ColumnDef::new(KybCases::SubmittedBy).uuid().not_null())
                    .col(ColumnDef::new(KybCases::OcrExtractedFields).json().null())
                    .col(ColumnDef::new(KybCases::PartnerDecision).string_len(32).null())
                    .col(ColumnDef::new(KybCases::RejectionReason).text().null())
                    .col(ColumnDef::new(KybCases::SubmittedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(KybCases::ResolvedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(KybCases::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_kc_operator").table(KybCases::Table).col(KybCases::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_kc_status").table(KybCases::Table).col(KybCases::Status).to_owned()).await
    }

    // ── BC-03: AmlAlert ─────────────────────────────────────────────────
    async fn create_aml_alerts(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AmlAlerts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AmlAlerts::AlertId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(AmlAlerts::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(AmlAlerts::TransactionId).uuid().not_null())
                    .col(ColumnDef::new(AmlAlerts::AlertType).string_len(64).not_null())
                    .col(ColumnDef::new(AmlAlerts::Severity).string_len(16).not_null())
                    .col(ColumnDef::new(AmlAlerts::RuleId).string_len(64).not_null())
                    .col(ColumnDef::new(AmlAlerts::Details).json().not_null().default("{}"))
                    .col(ColumnDef::new(AmlAlerts::Status).string_len(16).not_null())
                    .col(ColumnDef::new(AmlAlerts::ReviewedBy).uuid().null())
                    .col(ColumnDef::new(AmlAlerts::ReviewedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(AmlAlerts::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_aa_operator").table(AmlAlerts::Table).col(AmlAlerts::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_aa_status").table(AmlAlerts::Table).col(AmlAlerts::Status).to_owned()).await
    }

    // ── BC-11: PaymentLink ──────────────────────────────────────────────
    async fn create_payment_links(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentLinks::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PaymentLinks::LinkId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(PaymentLinks::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(PaymentLinks::AmountMinorUnits).big_integer().not_null())
                    .col(ColumnDef::new(PaymentLinks::Currency).string_len(3).not_null())
                    .col(ColumnDef::new(PaymentLinks::Description).string_len(255).not_null())
                    .col(ColumnDef::new(PaymentLinks::Status).string_len(32).not_null())
                    .col(ColumnDef::new(PaymentLinks::CheckoutToken).string_len(128).not_null())
                    .col(ColumnDef::new(PaymentLinks::ExpiresAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(PaymentLinks::MaxUses).integer().not_null().default(1))
                    .col(ColumnDef::new(PaymentLinks::UseCount).integer().not_null().default(0))
                    .col(ColumnDef::new(PaymentLinks::SuccessUrl).string_len(512).not_null())
                    .col(ColumnDef::new(PaymentLinks::CancelUrl).string_len(512).not_null())
                    .col(ColumnDef::new(PaymentLinks::PaymentIntentIds).json().not_null().default("[]"))
                    .col(ColumnDef::new(PaymentLinks::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(PaymentLinks::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_pl_operator").table(PaymentLinks::Table).col(PaymentLinks::OperatorId).to_owned()).await
    }

    // ── BC-14: Notification ──────────────────────────────────────────────
    async fn create_notifications(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notifications::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Notifications::NotificationId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Notifications::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(Notifications::Channel).string_len(32).not_null())
                    .col(ColumnDef::new(Notifications::Recipient).string_len(255).not_null())
                    .col(ColumnDef::new(Notifications::TemplateId).string_len(128).not_null())
                    .col(ColumnDef::new(Notifications::PayloadJson).json().not_null().default("{}"))
                    .col(ColumnDef::new(Notifications::Subject).string_len(255).null())
                    .col(ColumnDef::new(Notifications::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Notifications::RetryCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Notifications::MaxRetries).integer().not_null().default(3))
                    .col(ColumnDef::new(Notifications::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Notifications::SentAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Notifications::LastError).text().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_notif_operator").table(Notifications::Table).col(Notifications::OperatorId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_notif_status").table(Notifications::Table).col(Notifications::Status).to_owned()).await
    }

    // ── BC-13: Document ─────────────────────────────────────────────────
    async fn create_documents(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Documents::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Documents::DocumentId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Documents::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(Documents::Category).string_len(64).not_null())
                    .col(ColumnDef::new(Documents::Filename).string_len(255).not_null())
                    .col(ColumnDef::new(Documents::ContentType).string_len(128).not_null())
                    .col(ColumnDef::new(Documents::SizeBytes).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Documents::StorageKey).string_len(512).not_null())
                    .col(ColumnDef::new(Documents::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Documents::OcrResult).json().null())
                    .col(ColumnDef::new(Documents::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_doc_operator").table(Documents::Table).col(Documents::OperatorId).to_owned()).await
    }

    // ── BC-12: RiskAssessment ──────────────────────────────────────────
    async fn create_risk_assessments(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RiskAssessments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RiskAssessments::RiskAssessmentId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(RiskAssessments::PaymentIntentId).uuid().not_null())
                    .col(ColumnDef::new(RiskAssessments::RiskScore).double().not_null())
                    .col(ColumnDef::new(RiskAssessments::RiskLevel).string_len(16).not_null())
                    .col(ColumnDef::new(RiskAssessments::RiskFactors).json().not_null().default("[]"))
                    .col(ColumnDef::new(RiskAssessments::RuleVersion).string_len(32).not_null())
                    .col(ColumnDef::new(RiskAssessments::AssessedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_ra_pi").table(RiskAssessments::Table).col(RiskAssessments::PaymentIntentId).to_owned()).await
    }

    // ── SagaCoordinator: SagaExecution ──────────────────────────────────
    async fn create_saga_executions(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SagaExecutions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SagaExecutions::SagaId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SagaExecutions::SagaType).string_len(64).not_null())
                    .col(ColumnDef::new(SagaExecutions::AggregateId).uuid().not_null())
                    .col(ColumnDef::new(SagaExecutions::Status).string_len(32).not_null())
                    .col(ColumnDef::new(SagaExecutions::Steps).json().not_null().default("[]"))
                    .col(ColumnDef::new(SagaExecutions::CurrentStep).integer().not_null().default(0))
                    .col(ColumnDef::new(SagaExecutions::CompensationRunning).boolean().not_null().default(false))
                    .col(ColumnDef::new(SagaExecutions::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(SagaExecutions::UpdatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(SagaExecutions::CompletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_saga_status").table(SagaExecutions::Table).col(SagaExecutions::Status).to_owned()).await
    }

    // ── Scheduler: ScheduledJob ─────────────────────────────────────────
    async fn create_scheduled_jobs(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ScheduledJobs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ScheduledJobs::JobId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ScheduledJobs::JobType).string_len(64).not_null())
                    .col(ColumnDef::new(ScheduledJobs::ScheduleExpr).string_len(64).not_null())
                    .col(ColumnDef::new(ScheduledJobs::PayloadJson).json().not_null().default("{}"))
                    .col(ColumnDef::new(ScheduledJobs::Status).string_len(16).not_null())
                    .col(ColumnDef::new(ScheduledJobs::MaxRetries).integer().not_null().default(3))
                    .col(ColumnDef::new(ScheduledJobs::RetryCount).integer().not_null().default(0))
                    .col(ColumnDef::new(ScheduledJobs::LastRunAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(ScheduledJobs::NextRunAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(ScheduledJobs::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(ScheduledJobs::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_sched_next_run").table(ScheduledJobs::Table).col(ScheduledJobs::NextRunAt).to_owned()).await
    }

    // ── MerchantConnectorOnboarding: OnboardingRequest ─────────────────
    async fn create_onboarding_requests(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OnboardingRequests::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OnboardingRequests::OnboardingId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(OnboardingRequests::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(OnboardingRequests::ConnectorId).string_len(64).not_null())
                    .col(ColumnDef::new(OnboardingRequests::Environment).string_len(16).not_null())
                    .col(ColumnDef::new(OnboardingRequests::Status).string_len(32).not_null())
                    .col(ColumnDef::new(OnboardingRequests::TestResult).string_len(32).null())
                    .col(ColumnDef::new(OnboardingRequests::ErrorMessage).text().null())
                    .col(ColumnDef::new(OnboardingRequests::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(OnboardingRequests::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_or_operator").table(OnboardingRequests::Table).col(OnboardingRequests::OperatorId).to_owned()).await
    }

    // ── IAM: PendingChange ──────────────────────────────────────────────
    async fn create_pending_changes(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PendingChanges::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PendingChanges::ChangeId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(PendingChanges::ChangeType).string_len(64).not_null())
                    .col(ColumnDef::new(PendingChanges::MakerId).uuid().not_null())
                    .col(ColumnDef::new(PendingChanges::CheckerId).uuid().null())
                    .col(ColumnDef::new(PendingChanges::Payload).binary().not_null())
                    .col(ColumnDef::new(PendingChanges::Status).string_len(32).not_null())
                    .col(ColumnDef::new(PendingChanges::MakerNote).text().null())
                    .col(ColumnDef::new(PendingChanges::CheckerNote).text().null())
                    .col(ColumnDef::new(PendingChanges::RequestedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(PendingChanges::ReviewedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(PendingChanges::ExpiresAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await
    }

    // ── Webhook: WebhookSubscription ─────────────────────────────────────
    async fn create_webhook_subscriptions(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookSubscriptions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(WebhookSubscriptions::SubscriptionId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(WebhookSubscriptions::OperatorId).uuid().not_null())
                    .col(ColumnDef::new(WebhookSubscriptions::Url).string_len(512).not_null())
                    .col(ColumnDef::new(WebhookSubscriptions::EventTypes).json().not_null().default("[]"))
                    .col(ColumnDef::new(WebhookSubscriptions::SecretHash).binary().not_null())
                    .col(ColumnDef::new(WebhookSubscriptions::Status).string_len(16).not_null())
                    .col(ColumnDef::new(WebhookSubscriptions::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(WebhookSubscriptions::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_ws_operator").table(WebhookSubscriptions::Table).col(WebhookSubscriptions::OperatorId).to_owned()).await
    }

    // ── Webhook: WebhookDelivery ─────────────────────────────────────────
    async fn create_webhook_deliveries(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookDeliveries::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(WebhookDeliveries::DeliveryId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(WebhookDeliveries::SubscriptionId).uuid().not_null())
                    .col(ColumnDef::new(WebhookDeliveries::EventType).string_len(64).not_null())
                    .col(ColumnDef::new(WebhookDeliveries::Payload).text().not_null())
                    .col(ColumnDef::new(WebhookDeliveries::Signature).string_len(128).not_null())
                    .col(ColumnDef::new(WebhookDeliveries::Status).string_len(32).not_null())
                    .col(ColumnDef::new(WebhookDeliveries::AttemptCount).integer().not_null().default(0))
                    .col(ColumnDef::new(WebhookDeliveries::LastAttemptAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(WebhookDeliveries::NextRetryAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(WebhookDeliveries::ResponseStatusCode).small_integer().null())
                    .col(ColumnDef::new(WebhookDeliveries::ResponseBody).text().null())
                    .col(ColumnDef::new(WebhookDeliveries::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_wd_subscription").table(WebhookDeliveries::Table).col(WebhookDeliveries::SubscriptionId).to_owned()).await?;
        manager.create_index(Index::create().name("idx_wd_retry").table(WebhookDeliveries::Table).col(WebhookDeliveries::NextRetryAt).to_owned()).await
    }
}

// ─── Column Identifiers ──────────────────────────────────────────────────────

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

#[derive(DeriveIden)]
enum KybCases {
    Table,
    KybCaseId,
    OperatorId,
    Status,
    DocumentIds,
    SubmittedBy,
    OcrExtractedFields,
    PartnerDecision,
    RejectionReason,
    SubmittedAt,
    ResolvedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AmlAlerts {
    Table,
    AlertId,
    OperatorId,
    TransactionId,
    AlertType,
    Severity,
    RuleId,
    Details,
    Status,
    ReviewedBy,
    ReviewedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum PaymentLinks {
    Table,
    LinkId,
    OperatorId,
    AmountMinorUnits,
    Currency,
    Description,
    Status,
    CheckoutToken,
    ExpiresAt,
    MaxUses,
    UseCount,
    SuccessUrl,
    CancelUrl,
    PaymentIntentIds,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Notifications {
    Table,
    NotificationId,
    OperatorId,
    Channel,
    Recipient,
    TemplateId,
    PayloadJson,
    Subject,
    Status,
    RetryCount,
    MaxRetries,
    CreatedAt,
    SentAt,
    LastError,
}

#[derive(DeriveIden)]
enum Documents {
    Table,
    DocumentId,
    OperatorId,
    Category,
    Filename,
    ContentType,
    SizeBytes,
    StorageKey,
    Status,
    OcrResult,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RiskAssessments {
    Table,
    RiskAssessmentId,
    PaymentIntentId,
    RiskScore,
    RiskLevel,
    RiskFactors,
    RuleVersion,
    AssessedAt,
}

#[derive(DeriveIden)]
enum SagaExecutions {
    Table,
    SagaId,
    SagaType,
    AggregateId,
    Status,
    Steps,
    CurrentStep,
    CompensationRunning,
    CreatedAt,
    UpdatedAt,
    CompletedAt,
}

#[derive(DeriveIden)]
enum ScheduledJobs {
    Table,
    JobId,
    JobType,
    ScheduleExpr,
    PayloadJson,
    Status,
    MaxRetries,
    RetryCount,
    LastRunAt,
    NextRunAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum OnboardingRequests {
    Table,
    OnboardingId,
    OperatorId,
    ConnectorId,
    Environment,
    Status,
    TestResult,
    ErrorMessage,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PendingChanges {
    Table,
    ChangeId,
    ChangeType,
    MakerId,
    CheckerId,
    Payload,
    Status,
    MakerNote,
    CheckerNote,
    RequestedAt,
    ReviewedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum WebhookSubscriptions {
    Table,
    SubscriptionId,
    OperatorId,
    Url,
    EventTypes,
    SecretHash,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum WebhookDeliveries {
    Table,
    DeliveryId,
    SubscriptionId,
    EventType,
    Payload,
    Signature,
    Status,
    AttemptCount,
    LastAttemptAt,
    NextRetryAt,
    ResponseStatusCode,
    ResponseBody,
    CreatedAt,
}
