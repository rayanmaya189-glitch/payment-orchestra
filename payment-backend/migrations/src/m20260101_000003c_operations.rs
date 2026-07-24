//! Migration M003c — Operations Tables
//!
//! Creates operations-related tables:
//! - risk_assessments (BC-12)
//! - saga_executions (saga-coordinator)
//! - scheduled_jobs (scheduler)
//! - onboarding_requests (merchant-connector-onboarding)
//! - pending_changes (iam-service)
//! - webhook_subscriptions (webhook)
//! - webhook_deliveries (webhook)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
        Ok(())
    }
}

impl Migration {
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
