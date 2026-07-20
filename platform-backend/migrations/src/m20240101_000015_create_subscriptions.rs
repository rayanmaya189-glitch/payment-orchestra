use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Subscription::Table)
                    .if_not_exists()
                    .col(pk_uuid(Subscription::SubscriptionId))
                    .col(uuid(Subscription::OperatorId).not_null())
                    .col(uuid(Subscription::CustomerId).not_null())
                    .col(string(Subscription::Status).not_null().default("active"))
                    .col(big_integer(Subscription::AmountMinorUnits).not_null())
                    .col(string(Subscription::Currency).not_null().default("AED"))
                    .col(string(Subscription::Interval).not_null())
                    .col(integer(Subscription::IntervalCount).not_null().default(1))
                    .col(string(Subscription::CurrentPeriodStart).not_null())
                    .col(string(Subscription::CurrentPeriodEnd).not_null())
                    .col(integer(Subscription::TrialPeriodDays).not_null().default(0))
                    .col(string(Subscription::PaymentMethodTokenId).null())
                    .col(uuid(Subscription::FailedPaymentIntentId).null())
                    .col(integer(Subscription::RetryCount).not_null().default(0))
                    .col(integer(Subscription::MaxRetries).not_null().default(3))
                    .col(timestamp_with_time_zone(Subscription::CanceledAt).null())
                    .col(timestamp_with_time_zone(Subscription::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Subscription::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_subscription_operator")
                    .table(Subscription::Table)
                    .col(Subscription::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_subscription_customer")
                    .table(Subscription::Table)
                    .col(Subscription::CustomerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_subscription_status")
                    .table(Subscription::Table)
                    .col(Subscription::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_subscription_period_end")
                    .table(Subscription::Table)
                    .col(Subscription::CurrentPeriodEnd)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Subscription::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Subscription {
    Table,
    SubscriptionId,
    OperatorId,
    CustomerId,
    Status,
    AmountMinorUnits,
    Currency,
    Interval,
    IntervalCount,
    CurrentPeriodStart,
    CurrentPeriodEnd,
    TrialPeriodDays,
    PaymentMethodTokenId,
    FailedPaymentIntentId,
    RetryCount,
    MaxRetries,
    CanceledAt,
    CreatedAt,
    UpdatedAt,
}
