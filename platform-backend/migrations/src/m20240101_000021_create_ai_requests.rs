use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AiRequest::Table)
                    .if_not_exists()
                    .col(pk_uuid(AiRequest::RequestId))
                    .col(uuid(AiRequest::PrincipalId).not_null())
                    .col(text(AiRequest::Prompt).not_null())
                    .col(string(AiRequest::Model).not_null())
                    .col(integer(AiRequest::MaxTokens).null())
                    .col(double(AiRequest::Temperature).null())
                    .col(text(AiRequest::RedactedPrompt).null())
                    .col(boolean(AiRequest::Blocked).not_null().default(false))
                    .col(text(AiRequest::BlockReason).null())
                    .col(integer(AiRequest::TokensUsed).null())
                    .col(double(AiRequest::CostUsd).null())
                    .col(string(AiRequest::Status).not_null().default("processing"))
                    .col(timestamp_with_time_zone(AiRequest::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(AiRequest::CompletedAt).null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_ai_request_principal")
                    .table(AiRequest::Table)
                    .col(AiRequest::PrincipalId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_ai_request_created_at")
                    .table(AiRequest::Table)
                    .col(AiRequest::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AiRequest::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum AiRequest {
    Table,
    RequestId,
    PrincipalId,
    Prompt,
    Model,
    MaxTokens,
    Temperature,
    RedactedPrompt,
    Blocked,
    BlockReason,
    TokensUsed,
    CostUsd,
    Status,
    CreatedAt,
    CompletedAt,
}
