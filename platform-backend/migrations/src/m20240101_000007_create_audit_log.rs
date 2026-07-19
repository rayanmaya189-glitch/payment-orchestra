use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SRS AUD-003: Append-only audit table with hash chaining (AUD-004)
        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuditLog::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::Timestamp)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::Service)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::PrincipalId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::ActorType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::Action)
                        .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::Resource)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::ResourceId)
                            .string_len(128)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::IpAddress)
                            .string_len(45)  // IPv6 max
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::UserAgent)
                            .string_len(512)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::RequestIds)
                            .json()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::BeforeSnapshot)
                            .json()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::AfterSnapshot)
                            .json()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::Outcome)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::ErrorMessage)
                            .text()
                            .null(),
                    )
                    // SRS AUD-004: Hash chaining for tamper-evidence
                    .col(
                        ColumnDef::new(AuditLog::PreviousEntryHash)
                            .binary_len(32)  // SHA-256
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuditLog::EntryHash)
                            .binary_len(32)  // SHA-256
                            .not_null(),
                    )
                    // SRS AUD-003: No UPDATE/DELETE — enforced via trigger
                    .col(
                        ColumnDef::new(AuditLog::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Indexes for efficient querying
                    .index(
                        Index::create()
                            .name("idx_audit_log_principal_id")
                            .col(AuditLog::PrincipalId),
                    )
                    .index(
                        Index::create()
                            .name("idx_audit_log_action")
                            .col(AuditLog::Action),
                    )
                    .index(
                        Index::create()
                            .name("idx_audit_log_resource")
                            .col(AuditLog::Resource),
                    )
                    .index(
                        Index::create()
                            .name("idx_audit_log_timestamp")
                            .col(AuditLog::Timestamp),
                    )
                    .index(
                        Index::create()
                            .name("idx_audit_log_service")
                            .col(AuditLog::Service),
                    )
                    .to_owned(),
            )
            .await?;

        // SRS AUD-003: Prevent UPDATE and DELETE on audit_log table
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE OR REPLACE FUNCTION prevent_audit_log_modification()
                 RETURNS TRIGGER AS $$
                 BEGIN
                     RAISE EXCEPTION 'Audit log records cannot be modified or deleted (SRS AUD-003)';
                     RETURN NULL;
                 END;
                 $$ LANGUAGE plpgsql;

                 CREATE TRIGGER trg_audit_log_no_update
                     BEFORE UPDATE ON audit_log
                     FOR EACH ROW EXECUTE FUNCTION prevent_audit_log_modification();

                 CREATE TRIGGER trg_audit_log_no_delete
                     BEFORE DELETE ON audit_log
                     FOR EACH ROW EXECUTE FUNCTION prevent_audit_log_modification();",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TRIGGER IF EXISTS trg_audit_log_no_delete ON audit_log;
                 DROP TRIGGER IF EXISTS trg_audit_log_no_update ON audit_log;
                 DROP FUNCTION IF EXISTS prevent_audit_log_modification;",
            )
            .await?;

        manager
            .drop_table(Table::drop().table(AuditLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    Timestamp,
    Service,
    PrincipalId,
    ActorType,
    Action,
    Resource,
    ResourceId,
    IpAddress,
    UserAgent,
    RequestIds,
    BeforeSnapshot,
    AfterSnapshot,
    Outcome,
    ErrorMessage,
    PreviousEntryHash,
    EntryHash,
    CreatedAt,
}
