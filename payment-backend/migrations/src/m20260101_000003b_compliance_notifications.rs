//! Migration M003b — Compliance & Notification Tables
//!
//! Creates compliance and notification tables:
//! - kyb_cases (BC-03)
//! - aml_alerts (BC-03)
//! - payment_links (BC-11)
//! - notifications (BC-14)
//! - documents (BC-13)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_kyb_cases(manager).await?;
        self.create_aml_alerts(manager).await?;
        self.create_payment_links(manager).await?;
        self.create_notifications(manager).await?;
        self.create_documents(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Documents::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Notifications::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PaymentLinks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AmlAlerts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(KybCases::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
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
