//! Migration M002b — Identity & Access Tables
//!
//! Creates identity and access management tables:
//! - operators (BC-02)
//! - principals (BC-01)
//! - api_keys (BC-01)
//! - role_assignments (BC-01)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_operators(manager).await?;
        self.create_principals(manager).await?;
        self.create_api_keys(manager).await?;
        self.create_role_assignments(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RoleAssignments::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ApiKeys::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Principals::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Operators::Table).to_owned()).await?;
        Ok(())
    }
}

impl Migration {
    async fn create_operators(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Operators::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Operators::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Operators::LegalName).string_len(255).not_null())
                    .col(ColumnDef::new(Operators::TradeLicenseNo).string_len(255).not_null())
                    .col(ColumnDef::new(Operators::Country).string_len(2).not_null().default("AE"))
                    .col(ColumnDef::new(Operators::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Operators::Subdomain).string_len(64).not_null())
                    .col(ColumnDef::new(Operators::Email).string_len(255).not_null())
                    .col(ColumnDef::new(Operators::VerificationTokenHash).string_len(255).null())
                    .col(ColumnDef::new(Operators::ProvisionedAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Operators::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Operators::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_op_subdomain").table(Operators::Table).col(Operators::Subdomain).unique().to_owned()).await
    }

    async fn create_principals(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Principals::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Principals::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Principals::PrincipalType).string_len(32).not_null())
                    .col(ColumnDef::new(Principals::Email).string_len(255).null())
                    .col(ColumnDef::new(Principals::PasswordHash).binary().null())
                    .col(ColumnDef::new(Principals::MfaEnrolled).boolean().not_null().default(false))
                    .col(ColumnDef::new(Principals::MfaMethod).string_len(32).null())
                    .col(ColumnDef::new(Principals::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Principals::FailedLoginAttempts).integer().not_null().default(0))
                    .col(ColumnDef::new(Principals::LockedUntil).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Principals::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Principals::LastLoginAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(Principals::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_principal_email").table(Principals::Table).col(Principals::Email).unique().to_owned()).await
    }

    async fn create_api_keys(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKeys::ApiKeyId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiKeys::PrincipalId).uuid().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).string_len(128).not_null())
                    .col(ColumnDef::new(ApiKeys::KeyPrefix).string_len(8).not_null())
                    .col(ColumnDef::new(ApiKeys::KeyHash).binary().not_null())
                    .col(ColumnDef::new(ApiKeys::Scopes).json().not_null().default("[]"))
                    .col(ColumnDef::new(ApiKeys::Status).string_len(16).not_null())
                    .col(ColumnDef::new(ApiKeys::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(ApiKeys::ExpiresAt).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(ApiKeys::LastUsedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        manager.create_index(Index::create().name("idx_ak_principal").table(ApiKeys::Table).col(ApiKeys::PrincipalId).to_owned()).await
    }

    async fn create_role_assignments(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RoleAssignments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RoleAssignments::PrincipalId).uuid().not_null())
                    .col(ColumnDef::new(RoleAssignments::Role).string_len(64).not_null())
                    .col(ColumnDef::new(RoleAssignments::AbacConditions).json().null())
                    .primary_key(
                        Index::create()
                            .col(RoleAssignments::PrincipalId)
                            .col(RoleAssignments::Role),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Operators {
    Table,
    Id,
    LegalName,
    TradeLicenseNo,
    Country,
    Status,
    Subdomain,
    Email,
    VerificationTokenHash,
    ProvisionedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Principals {
    Table,
    Id,
    PrincipalType,
    Email,
    PasswordHash,
    MfaEnrolled,
    MfaMethod,
    Status,
    FailedLoginAttempts,
    LockedUntil,
    CreatedAt,
    LastLoginAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    ApiKeyId,
    PrincipalId,
    Name,
    KeyPrefix,
    KeyHash,
    Scopes,
    Status,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum RoleAssignments {
    Table,
    PrincipalId,
    Role,
    AbacConditions,
}
