use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RoutingPolicy::Table)
                    .if_not_exists()
                    .col(pk_uuid(RoutingPolicy::RoutingPolicyId))
                    .col(uuid(RoutingPolicy::OperatorId).not_null())
                    .col(integer(RoutingPolicy::Version).not_null().default(1))
                    .col(string(RoutingPolicy::Status).not_null().default("active"))
                    .col(json(RoutingPolicy::Rules).not_null())
                    .col(json(RoutingPolicy::FailoverConfig).not_null())
                    .col(timestamp_with_time_zone(RoutingPolicy::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(RoutingPolicy::ActivatedAt).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_routing_policy_operator")
                    .table(RoutingPolicy::Table)
                    .col(RoutingPolicy::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RoutingAttempt::Table)
                    .if_not_exists()
                    .col(pk_uuid(RoutingAttempt::AttemptId))
                    .col(uuid(RoutingAttempt::PaymentIntentId).not_null())
                    .col(integer(RoutingAttempt::AttemptNumber).not_null())
                    .col(uuid(RoutingAttempt::AcquirerLinkId).not_null())
                    .col(string(RoutingAttempt::ConnectorId).not_null())
                    .col(string(RoutingAttempt::Status).not_null().default("pending"))
                    .col(string(RoutingAttempt::DeclineReason).null())
                    .col(string(RoutingAttempt::AcquirerReference).null())
                    .col(integer(RoutingAttempt::LatencyMs).not_null().default(0))
                    .col(timestamp_with_time_zone(RoutingAttempt::AttemptedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_routing_attempt_intent")
                            .from(RoutingAttempt::Table, RoutingAttempt::PaymentIntentId)
                            .to(PaymentIntent::Table, PaymentIntent::PaymentIntentId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_routing_attempt_intent")
                    .table(RoutingAttempt::Table)
                    .col(RoutingAttempt::PaymentIntentId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RoutingAttempt::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RoutingPolicy::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum RoutingPolicy {
    Table,
    RoutingPolicyId,
    OperatorId,
    Version,
    Status,
    Rules,
    FailoverConfig,
    CreatedAt,
    ActivatedAt,
}

#[derive(Iden)]
enum RoutingAttempt {
    Table,
    AttemptId,
    PaymentIntentId,
    AttemptNumber,
    AcquirerLinkId,
    ConnectorId,
    Status,
    DeclineReason,
    AcquirerReference,
    LatencyMs,
    AttemptedAt,
}

#[derive(Iden)]
enum PaymentIntent {
    Table,
    PaymentIntentId,
}
