use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notification::Table)
                    .if_not_exists()
                    .col(pk_uuid(Notification::NotificationId))
                    .col(uuid(Notification::OperatorId).not_null())
                    .col(string(Notification::NotificationType).not_null()) // email, sms, webhook
                    .col(string(Notification::Status).not_null().default("pending"))
                    .col(string(Notification::Recipient).not_null())
                    .col(string(Notification::Subject).null())
                    .col(text(Notification::Body).not_null())
                    .col(string(Notification::TemplateId).null())
                    .col(json(Notification::TemplateData).null())
                    .col(string(Notification::ProviderMessageId).null())
                    .col(string(Notification::ProviderStatus).null())
                    .col(integer(Notification::RetryCount).not_null().default(0))
                    .col(timestamp_with_time_zone(Notification::ScheduledAt).null())
                    .col(timestamp_with_time_zone(Notification::SentAt).null())
                    .col(timestamp_with_time_zone(Notification::DeliveredAt).null())
                    .col(timestamp_with_time_zone(Notification::FailedAt).null())
                    .col(text(Notification::ErrorMessage).null())
                    .col(timestamp_with_time_zone(Notification::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Notification::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_operator")
                    .table(Notification::Table)
                    .col(Notification::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_status")
                    .table(Notification::Table)
                    .col(Notification::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_recipient")
                    .table(Notification::Table)
                    .col(Notification::Recipient)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Notification::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Notification {
    Table,
    NotificationId,
    OperatorId,
    NotificationType,
    Status,
    Recipient,
    Subject,
    Body,
    TemplateId,
    TemplateData,
    ProviderMessageId,
    ProviderStatus,
    RetryCount,
    ScheduledAt,
    SentAt,
    DeliveredAt,
    FailedAt,
    ErrorMessage,
    CreatedAt,
    UpdatedAt,
}
