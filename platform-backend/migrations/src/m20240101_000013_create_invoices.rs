use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Invoice::Table)
                    .if_not_exists()
                    .col(pk_uuid(Invoice::InvoiceId))
                    .col(uuid(Invoice::OperatorId).not_null())
                    .col(string(Invoice::OrderReference).not_null().unique_key())
                    .col(string(Invoice::Status).not_null().default("draft"))
                    .col(big_integer(Invoice::TotalAmountMinorUnits).not_null())
                    .col(big_integer(Invoice::PaidAmountMinorUnits).not_null().default(0))
                    .col(string(Invoice::Currency).not_null().default("AED"))
                    .col(timestamp_with_time_zone(Invoice::DueDate).not_null())
                    .col(string(Invoice::RecipientEmail).null())
                    .col(json(Invoice::LineItems).not_null())
                    .col(json(Invoice::PaymentIntentIds).not_null().default("[]"))
                    .col(timestamp_with_time_zone(Invoice::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Invoice::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_invoice_operator")
                    .table(Invoice::Table)
                    .col(Invoice::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_invoice_status")
                    .table(Invoice::Table)
                    .col(Invoice::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Invoice::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Invoice {
    Table,
    InvoiceId,
    OperatorId,
    OrderReference,
    Status,
    TotalAmountMinorUnits,
    PaidAmountMinorUnits,
    Currency,
    DueDate,
    RecipientEmail,
    LineItems,
    PaymentIntentIds,
    CreatedAt,
    UpdatedAt,
}
