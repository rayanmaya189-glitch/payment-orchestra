#![allow(clippy::enum_variant_names)]
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Principal::Table)
                    .if_not_exists()
                    .col(pk_uuid(Principal::Id))
                    .col(string(Principal::PrincipalType).not_null())
                    .col(string(Principal::Email).null().unique_key())
                    .col(binary(Principal::PasswordHash).null())
                    .col(boolean(Principal::MfaEnrolled).not_null().default(false))
                    .col(string(Principal::MfaMethod).null())
                    .col(string(Principal::Status).not_null().default("active"))
                    .col(integer(Principal::FailedLoginAttempts).not_null().default(0))
                    .col(timestamp_with_time_zone(Principal::LockedUntil).null())
                    .col(timestamp_with_time_zone(Principal::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Principal::LastLoginAt).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_principal_email")
                    .table(Principal::Table)
                    .col(Principal::Email)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Principal::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Principal {
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
}
