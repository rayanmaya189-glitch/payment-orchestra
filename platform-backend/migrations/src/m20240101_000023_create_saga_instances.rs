use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SagaInstance::Table)
                    .if_not_exists()
                    .col(pk_uuid(SagaInstance::SagaId))
                    .col(string(SagaInstance::SagaType).not_null())
                    .col(string(SagaInstance::Status).not_null().default("running"))
                    .col(integer(SagaInstance::CurrentStep).not_null().default(1))
                    .col(integer(SagaInstance::TotalSteps).not_null())
                    .col(json(SagaInstance::Steps).not_null())
                    .col(json(SagaInstance::Payload).not_null())
                    .col(json(SagaInstance::CompensationData).null())
                    .col(timestamp_with_time_zone(SagaInstance::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(SagaInstance::UpdatedAt).not_null())
                    .col(timestamp_with_time_zone(SagaInstance::CompletedAt).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_saga_instance_status")
                    .table(SagaInstance::Table)
                    .col(SagaInstance::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_saga_instance_type")
                    .table(SagaInstance::Table)
                    .col(SagaInstance::SagaType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SagaInstance::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum SagaInstance {
    Table,
    SagaId,
    SagaType,
    Status,
    CurrentStep,
    TotalSteps,
    Steps,
    Payload,
    CompensationData,
    CreatedAt,
    UpdatedAt,
    CompletedAt,
}
