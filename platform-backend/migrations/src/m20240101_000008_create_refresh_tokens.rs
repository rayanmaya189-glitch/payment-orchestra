use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Persistent refresh token tracking (backup to Redis, for audit compliance)
        manager
            .create_table(
                Table::create()
                    .table(RefreshToken::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RefreshToken::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::PrincipalId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::Role)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::ClientFingerprint)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::Status)
                            .string_len(32)
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RefreshToken::RevokeReason)
                            .string_len(128)
                            .null(),
                    )
                    .index(
                        Index::create()
                            .name("idx_refresh_token_principal")
                            .col(RefreshToken::PrincipalId),
                    )
                    .index(
                        Index::create()
                            .name("idx_refresh_token_status")
                            .col(RefreshToken::Status),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RefreshToken::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum RefreshToken {
    Table,
    Id,
    PrincipalId,
    Role,
    ClientFingerprint,
    Status,
    CreatedAt,
    RevokedAt,
    RevokeReason,
}
