use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::{RiskAssessment, default_rules};
use crate::domain::rules::RiskAssessmentRepository;
use platform_error::PlatformError;

pub struct RiskServiceImpl { repo: Box<dyn RiskAssessmentRepository>, db: DatabaseConnection }
impl RiskServiceImpl {
    pub fn new(repo: Box<dyn RiskAssessmentRepository>, db: DatabaseConnection) -> Self { Self { repo, db } }
}

#[async_trait]
pub trait RiskService: Send + Sync {
    async fn assess(&self, cmd: AssessPaymentRiskCommand) -> Result<RiskAssessment, PlatformError>;
    async fn get_assessment(&self, id: Uuid) -> Result<RiskAssessment, PlatformError>;
}

#[async_trait]
impl RiskService for RiskServiceImpl {
    async fn assess(&self, cmd: AssessPaymentRiskCommand) -> Result<RiskAssessment, PlatformError> {
        let mut assessment = RiskAssessment::new(cmd.payment_intent_id, cmd.operator_id);
        assessment.ip_address = cmd.ip_address;
        assessment.user_agent = cmd.user_agent;
        let rules = default_rules();
        assessment.evaluate(&rules);
        self.repo.save(&assessment).await?;
        Ok(assessment)
    }

    async fn get_assessment(&self, id: Uuid) -> Result<RiskAssessment, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "risk_assessment".into(), id })
    }
}
