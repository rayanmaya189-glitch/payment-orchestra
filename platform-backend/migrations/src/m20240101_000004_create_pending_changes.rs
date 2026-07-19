use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PendingChange::Table)
                    .if_not_exists()
                    .col(pk_uuid(PendingChange::Id))
                    .col(string(PendingChange::ChangeType).not_null())
                    .col(uuid(PendingChange::MakerId).not_null())
                    .col(uuid(PendingChange::CheckerId).null())
                    .col(binary(PendingChange::Payload).not_null())
                    .col(string(PendingChange::Status).not_null().default("pending"))
                    .col(string(PendingChange::MakerNote).null())
                    .col(string(PendingChange::CheckerNote).null())
                    .col(timestamp_with_time_zone(PendingChange::RequestedAt).not_null())
                    .col(timestamp_with_time_zone(PendingChange::ReviewedAt).null())
                    .col(timestamp_with_time_zone(PendingChange::ExpiresAt).not_null())
                    .col(timestamp_with_time_zone(PendingChange::CreatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PendingChange::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum PendingChange {
    Table,
    Id,
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
    CreatedAt,
}
