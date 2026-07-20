use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Document::Table)
                    .if_not_exists()
                    .col(pk_uuid(Document::DocumentId))
                    .col(uuid(Document::OperatorId).not_null())
                    .col(string(Document::DocumentType).not_null()) // trade_license, passport, etc.
                    .col(string(Document::Status).not_null().default("uploaded"))
                    .col(string(Document::Filename).not_null())
                    .col(string(Document::ContentType).not_null())
                    .col(big_integer(Document::FileSize).not_null())
                    .col(string(Document::StorageKey).not_null())
                    .col(string(Document::FileHash).not_null())
                    .col(json(Document::OcrResult).null())
                    .col(json(Document::Metadata).null())
                    .col(string(Document::UploadedBy).not_null())
                    .col(string(Document::VerificationStatus).not_null().default("pending"))
                    .col(text(Document::VerificationNotes).null())
                    .col(timestamp_with_time_zone(Document::VerifiedAt).null())
                    .col(timestamp_with_time_zone(Document::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(Document::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_document_operator")
                    .table(Document::Table)
                    .col(Document::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_document_type")
                    .table(Document::Table)
                    .col(Document::DocumentType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_document_status")
                    .table(Document::Table)
                    .col(Document::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Document::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Document {
    Table,
    DocumentId,
    OperatorId,
    DocumentType,
    Status,
    Filename,
    ContentType,
    FileSize,
    StorageKey,
    FileHash,
    OcrResult,
    Metadata,
    UploadedBy,
    VerificationStatus,
    VerificationNotes,
    VerifiedAt,
    CreatedAt,
    UpdatedAt,
}
