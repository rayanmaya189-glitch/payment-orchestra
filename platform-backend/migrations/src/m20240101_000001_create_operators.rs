use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Operator::Table)
                    .if_not_exists()
                    .col(pk_uuid(Operator::Id))
                    .col(string(Operator::LegalName).not_null())
                    .col(string(Operator::TradeLicenseNo).not_null().unique_key())
                    .col(string(Operator::Country).not_null().default("AE"))
                    .col(string(Operator::Status).not_null().default("pending"))
                    .col(string(Operator::Subdomain).not_null().unique_key())
                    .col(string(Operator::Email).not_null())
                    .col(timestamp_with_time_zone(Operator::ProvisionedAt).null())
                    .col(timestamp_with_time_zone(Operator::CreatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_operator_status")
                    .table(Operator::Table)
                    .col(Operator::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Operator::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Operator {
    Table,
    Id,
    LegalName,
    TradeLicenseNo,
    Country,
    Status,
    Subdomain,
    Email,
    ProvisionedAt,
    CreatedAt,
}
