//! Migration M001 — Core Infrastructure Tables
//!
//! Creates the foundational tables shared across services:
//! - event_store: Append-only event log for event-sourced aggregates
//! - aggregate_snapshots: Materialized snapshots for long event streams
//! - outbox_entries: Transactional outbox for reliable event publishing
//! - audit_log: Append-only audit trail for compliance

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        self.create_event_store(manager).await?;
        self.create_aggregate_snapshots(manager).await?;
        self.create_outbox_entries(manager).await?;
        self.create_audit_log(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(AuditLog::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(OutboxEntries::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AggregateSnapshots::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(EventStore::Table).to_owned()).await?;
        Ok(())
    }
}

// ── Table Definitions ────────────────────────────────────────────────────────

impl Migration {
    /// DB-001: Event store — append-only log for event-sourced aggregates.
    /// Composite PK: (aggregate_type, aggregate_id, event_sequence).
    async fn create_event_store(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EventStore::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventStore::AggregateType)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventStore::AggregateId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventStore::EventSequence)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventStore::EventId).uuid().not_null())
                    .col(
                        ColumnDef::new(EventStore::EventType)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventStore::EventVersion).small_integer().not_null().default(1))
                    .col(
                        ColumnDef::new(EventStore::OccurredAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventStore::ActorType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventStore::ActorId).uuid().null())
                    .col(ColumnDef::new(EventStore::CausationId).uuid().null())
                    .col(
                        ColumnDef::new(EventStore::CorrelationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventStore::Payload).binary().not_null())
                    .col(ColumnDef::new(EventStore::Checksum).binary().null())
                    .col(
                        ColumnDef::new(EventStore::Encrypted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .primary_key(
                        Index::create()
                            .col(EventStore::AggregateType)
                            .col(EventStore::AggregateId)
                            .col(EventStore::EventSequence),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint on event_id for idempotent append
        manager
            .create_index(
                Index::create()
                    .name("idx_event_store_event_id_uniq")
                    .table(EventStore::Table)
                    .col(EventStore::EventId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index for correlation ID queries
        manager
            .create_index(
                Index::create()
                    .name("idx_event_store_correlation_id")
                    .table(EventStore::Table)
                    .col(EventStore::CorrelationId)
                    .to_owned(),
            )
            .await?;

        // Index for time-range queries
        manager
            .create_index(
                Index::create()
                    .name("idx_event_store_occurred_at")
                    .table(EventStore::Table)
                    .col(EventStore::OccurredAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    /// DB-003: Aggregate snapshots — periodic materialized state for long event streams.
    async fn create_aggregate_snapshots(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AggregateSnapshots::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AggregateSnapshots::AggregateType)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AggregateSnapshots::AggregateId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AggregateSnapshots::AsOfSequence)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AggregateSnapshots::State)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AggregateSnapshots::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(AggregateSnapshots::AggregateType)
                            .col(AggregateSnapshots::AggregateId)
                            .col(AggregateSnapshots::AsOfSequence),
                    )
                    .to_owned(),
            )
            .await
    }

    /// DB-005/006: Outbox entries — transactional outbox for reliable event publishing.
    async fn create_outbox_entries(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OutboxEntries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OutboxEntries::EntryId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::AggregateType)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::AggregateId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::EventType)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::EventVersion)
                            .small_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::Subject)
                            .string_len(255)
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::Payload)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::Published)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEntries::PublishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for relay — find unpublished entries efficiently
        manager
            .create_index(
                Index::create()
                    .name("idx_outbox_unpublished")
                    .table(OutboxEntries::Table)
                    .col(OutboxEntries::Published)
                    .col(OutboxEntries::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    /// DB-004: Audit log — append-only trail for compliance.
    async fn create_audit_log(&self, manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                    .col(ColumnDef::new(AuditLog::ActorId).uuid().null())
                    .col(ColumnDef::new(AuditLog::ActorType).string_len(32).null())
                    .col(
                        ColumnDef::new(AuditLog::Action)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(ColumnDef::new(AuditLog::ResourceType).string_len(64).null())
                    .col(ColumnDef::new(AuditLog::ResourceId).uuid().null())
                    .col(ColumnDef::new(AuditLog::BeforeState).json().null())
                    .col(ColumnDef::new(AuditLog::AfterState).json().null())
                    .col(
                        ColumnDef::new(AuditLog::OccurredAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AuditLog::SourceIp).string_len(45).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_actor")
                    .table(AuditLog::Table)
                    .col(AuditLog::ActorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_occurred_at")
                    .table(AuditLog::Table)
                    .col(AuditLog::OccurredAt)
                    .to_owned(),
            )
            .await
    }
}

// ─── Column Identifiers ──────────────────────────────────────────────────────

#[derive(DeriveIden)]
enum EventStore {
    Table,
    AggregateType,
    AggregateId,
    EventSequence,
    EventId,
    EventType,
    EventVersion,
    OccurredAt,
    ActorType,
    ActorId,
    CausationId,
    CorrelationId,
    Payload,
    Checksum,
    Encrypted,
}

#[derive(DeriveIden)]
enum AggregateSnapshots {
    Table,
    AggregateType,
    AggregateId,
    AsOfSequence,
    State,
    CreatedAt,
}

#[derive(DeriveIden)]
enum OutboxEntries {
    Table,
    EntryId,
    AggregateType,
    AggregateId,
    EventType,
    EventVersion,
    Subject,
    Payload,
    Published,
    CreatedAt,
    PublishedAt,
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    ActorId,
    ActorType,
    Action,
    ResourceType,
    ResourceId,
    BeforeState,
    AfterState,
    OccurredAt,
    SourceIp,
}
