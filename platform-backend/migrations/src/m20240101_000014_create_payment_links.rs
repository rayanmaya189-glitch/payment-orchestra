use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PaymentLink::Table)
                    .if_not_exists()
                    .col(pk_uuid(PaymentLink::LinkId))
                    .col(uuid(PaymentLink::OperatorId).not_null())
                    .col(string(PaymentLink::Status).not_null().default("active"))
                    .col(string(PaymentLink::Description).not_null())
                    .col(string(PaymentLink::MerchantName).not_null())
                    .col(big_integer(PaymentLink::AmountMinorUnits).not_null())
                    .col(string(PaymentLink::Currency).not_null().default("AED"))
                    .col(integer(PaymentLink::MaxUses).null())
                    .col(integer(PaymentLink::CurrentUses).not_null().default(0))
                    .col(timestamp_with_time_zone(PaymentLink::ExpiresAt).null())
                    .col(string(PaymentLink::PublicToken).not_null().unique_key())
                    .col(json(PaymentLink::Metadata).null())
                    .col(timestamp_with_time_zone(PaymentLink::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(PaymentLink::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_payment_link_operator")
                    .table(PaymentLink::Table)
                    .col(PaymentLink::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_payment_link_token")
                    .table(PaymentLink::Table)
                    .col(PaymentLink::PublicToken)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PaymentLink::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum PaymentLink {
    Table,
    LinkId,
    OperatorId,
    Status,
    Description,
    MerchantName,
    AmountMinorUnits,
    Currency,
    MaxUses,
    CurrentUses,
    ExpiresAt,
    PublicToken,
    Metadata,
    CreatedAt,
    UpdatedAt,
}
