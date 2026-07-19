use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKey::Table)
                    .if_not_exists()
                    .col(pk_uuid(ApiKey::Id))
                    .col(uuid(ApiKey::PrincipalId).not_null())
                    .col(string(ApiKey::Name).not_null())
                    .col(binary(ApiKey::KeyHash).not_null())
                    .col(json_binary(ApiKey::Scopes).not_null())
                    .col(json_binary(ApiKey::AcquirerLinkIds).null())
                    .col(timestamp_with_time_zone(ApiKey::ExpiresAt).not_null())
                    .col(timestamp_with_time_zone(ApiKey::RevokedAt).null())
                    .col(timestamp_with_time_zone(ApiKey::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_key_principal")
                            .from(ApiKey::Table, ApiKey::PrincipalId)
                            .to(Principal::Table, Principal::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_api_key_principal")
                    .table(ApiKey::Table)
                    .col(ApiKey::PrincipalId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKey::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum ApiKey {
    Table,
    Id,
    PrincipalId,
    Name,
    KeyHash,
    Scopes,
    AcquirerLinkIds,
    ExpiresAt,
    RevokedAt,
    CreatedAt,
}

#[derive(Iden)]
enum Principal {
    Table,
    Id,
}
