use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RiskAssessment::Table)
                    .if_not_exists()
                    .col(pk_uuid(RiskAssessment::AssessmentId))
                    .col(uuid(RiskAssessment::PaymentIntentId).not_null())
                    .col(uuid(RiskAssessment::OperatorId).not_null())
                    .col(double(RiskAssessment::Score).not_null().default(0.0))
                    .col(string(RiskAssessment::Decision).not_null().default("allow"))
                    .col(json(RiskAssessment::Factors).not_null())
                    .col(string(RiskAssessment::IpAddress).null())
                    .col(string(RiskAssessment::UserAgent).null())
                    .col(string(RiskAssessment::DeviceFingerprint).null())
                    .col(double(RiskAssessment::VelocityScore).not_null().default(0.0))
                    .col(double(RiskAssessment::GeoScore).not_null().default(0.0))
                    .col(double(RiskAssessment::BehaviorScore).not_null().default(0.0))
                    .col(boolean(RiskAssessment::IsWhitelisted).not_null().default(false))
                    .col(boolean(RiskAssessment::IsBlacklisted).not_null().default(false))
                    .col(timestamp_with_time_zone(RiskAssessment::CreatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_risk_assessment_payment_intent")
                    .table(RiskAssessment::Table)
                    .col(RiskAssessment::PaymentIntentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_risk_assessment_operator")
                    .table(RiskAssessment::Table)
                    .col(RiskAssessment::OperatorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_risk_assessment_decision")
                    .table(RiskAssessment::Table)
                    .col(RiskAssessment::Decision)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RiskAssessment::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum RiskAssessment {
    Table,
    AssessmentId,
    PaymentIntentId,
    OperatorId,
    Score,
    Decision,
    Factors,
    IpAddress,
    UserAgent,
    DeviceFingerprint,
    VelocityScore,
    GeoScore,
    BehaviorScore,
    IsWhitelisted,
    IsBlacklisted,
    CreatedAt,
}
