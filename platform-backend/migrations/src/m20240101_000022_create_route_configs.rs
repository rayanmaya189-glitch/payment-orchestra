use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RouteConfig::Table)
                    .if_not_exists()
                    .col(pk_uuid(RouteConfig::RouteId))
                    .col(string(RouteConfig::PathPrefix).not_null().unique_key())
                    .col(string(RouteConfig::TargetService).not_null())
                    .col(string(RouteConfig::TargetUrl).not_null())
                    .col(boolean(RouteConfig::AuthRequired).not_null().default(true))
                    .col(integer(RouteConfig::RateLimit).null())
                    .col(timestamp_with_time_zone(RouteConfig::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(RouteConfig::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_route_config_prefix")
                    .table(RouteConfig::Table)
                    .col(RouteConfig::PathPrefix)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RouteConfig::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum RouteConfig {
    Table,
    RouteId,
    PathPrefix,
    TargetService,
    TargetUrl,
    AuthRequired,
    RateLimit,
    CreatedAt,
    UpdatedAt,
}
