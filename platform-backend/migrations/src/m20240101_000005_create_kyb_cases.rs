use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KybCase::Table)
                    .if_not_exists()
                    .col(pk_uuid(KybCase::Id))
                    .col(uuid(KybCase::OperatorId).not_null())
                    .col(string(KybCase::Status).not_null().default("submitted"))
                    .col(uuid(KybCase::AssignedComplianceOfficer).null())
                    .col(double(KybCase::RiskScore).null())
                    .col(string(KybCase::Decision).null())
                    .col(string(KybCase::DecisionReason).null())
                    .col(string(KybCase::Notes).null())
                    .col(timestamp_with_time_zone(KybCase::SubmittedAt).not_null())
                    .col(timestamp_with_time_zone(KybCase::ReviewedAt).null())
                    .col(timestamp_with_time_zone(KybCase::DecidedAt).null())
                    .col(timestamp_with_time_zone(KybCase::CreatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(KybDocument::Table)
                    .if_not_exists()
                    .col(pk_uuid(KybDocument::Id))
                    .col(uuid(KybDocument::KybCaseId).not_null())
                    .col(string(KybDocument::DocumentType).not_null())
                    .col(string(KybDocument::FileKey).not_null())
                    .col(string(KybDocument::FileHash).not_null())
                    .col(boolean(KybDocument::Verified).not_null().default(false))
                    .col(timestamp_with_time_zone(KybDocument::UploadedAt).not_null())
                    .col(timestamp_with_time_zone(KybDocument::VerifiedAt).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_kyb_doc_case")
                            .from(KybDocument::Table, KybDocument::KybCaseId)
                            .to(KybCase::Table, KybCase::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_kyb_case_operator")
                    .table(KybCase::Table)
                    .col(KybCase::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_kyb_case_status")
                    .table(KybCase::Table)
                    .col(KybCase::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(KybDocument::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(KybCase::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum KybCase {
    Table,
    Id,
    OperatorId,
    Status,
    AssignedComplianceOfficer,
    RiskScore,
    Decision,
    DecisionReason,
    Notes,
    SubmittedAt,
    ReviewedAt,
    DecidedAt,
    CreatedAt,
}

#[derive(Iden)]
enum KybDocument {
    Table,
    Id,
    KybCaseId,
    DocumentType,
    FileKey,
    FileHash,
    Verified,
    UploadedAt,
    VerifiedAt,
}
