use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Outbox::Table)
                    .if_not_exists()
                    .col(pk_uuid(Outbox::Id))
                    .col(string(Outbox::AggregateType).not_null())
                    .col(uuid(Outbox::AggregateId).not_null())
                    .col(string(Outbox::EventType).not_null())
                    .col(small_integer(Outbox::EventVersion).not_null().default(1))
                    .col(binary(Outbox::Payload).not_null())
                    .col(timestamp_with_time_zone(Outbox::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Outbox::PublishedAt).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_outbox_unpublished")
                    .table(Outbox::Table)
                    .col(Outbox::PublishedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Outbox::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Outbox {
    Table,
    Id,
    AggregateType,
    AggregateId,
    EventType,
    EventVersion,
    Payload,
    CreatedAt,
    PublishedAt,
}
