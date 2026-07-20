use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GatewayProfile::Table)
                    .if_not_exists()
                    .col(pk_uuid(GatewayProfile::ProfileId))
                    .col(string(GatewayProfile::ConnectorId).not_null())
                    .col(uuid(GatewayProfile::OperatorId).not_null())
                    .col(string(GatewayProfile::Status).not_null().default("active"))
                    .col(string(GatewayProfile::BaseUrl).not_null())
                    .col(string(GatewayProfile::MerchantAcquirerLinkId).not_null())
                    .col(json(GatewayProfile::EnabledCardSchemes).not_null())
                    .col(json(GatewayProfile::EnabledCurrencies).not_null())
                    .col(big_integer(GatewayProfile::MinTransactionAmount).null())
                    .col(big_integer(GatewayProfile::MaxTransactionAmount).null())
                    .col(integer(GatewayProfile::RoutingPriority).not_null().default(1))
                    .col(boolean(GatewayProfile::SupportsPartialCapture).not_null().default(false))
                    .col(boolean(GatewayProfile::Supports3DS).not_null().default(true))
                    .col(string(GatewayProfile::CredentialEncrypted).null())
                    .col(timestamp_with_time_zone(GatewayProfile::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(GatewayProfile::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gateway_profile_operator")
                    .table(GatewayProfile::Table)
                    .col(GatewayProfile::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gateway_profile_connector")
                    .table(GatewayProfile::Table)
                    .col(GatewayProfile::ConnectorId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GatewayProfile::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum GatewayProfile {
    Table,
    ProfileId,
    ConnectorId,
    OperatorId,
    Status,
    BaseUrl,
    MerchantAcquirerLinkId,
    EnabledCardSchemes,
    EnabledCurrencies,
    MinTransactionAmount,
    MaxTransactionAmount,
    RoutingPriority,
    SupportsPartialCapture,
    Supports3DS,
    CredentialEncrypted,
    CreatedAt,
    UpdatedAt,
}
