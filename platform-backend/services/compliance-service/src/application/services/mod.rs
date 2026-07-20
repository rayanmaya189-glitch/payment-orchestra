use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::KybCase;
use crate::domain::rules::KybCaseRepository;
use platform_error::PlatformError;

pub struct ComplianceServiceImpl { repo: Box<dyn KybCaseRepository>, db: DatabaseConnection }
impl ComplianceServiceImpl {
    pub fn new(repo: Box<dyn KybCaseRepository>, db: DatabaseConnection) -> Self { Self { repo, db } }
}

#[async_trait]
pub trait ComplianceService: Send + Sync {
    async fn submit_kyb(&self, cmd: SubmitKybCommand) -> Result<Uuid, PlatformError>;
    async fn assign_officer(&self, cmd: AssignOfficerCommand) -> Result<(), PlatformError>;
    async fn decide(&self, cmd: DecideKybCommand) -> Result<(), PlatformError>;
    async fn get_case(&self, cmd: GetKybCaseCommand) -> Result<KybCase, PlatformError>;
}

#[async_trait]
impl ComplianceService for ComplianceServiceImpl {
    async fn submit_kyb(&self, cmd: SubmitKybCommand) -> Result<Uuid, PlatformError> {
        let mut case = KybCase::new(cmd.operator_id);
        self.repo.save(&case).await?;
        Ok(case.case_id)
    }
    async fn assign_officer(&self, cmd: AssignOfficerCommand) -> Result<(), PlatformError> {
        let mut case = self.repo.find_by_id(cmd.case_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "kyb_case".into(), id: cmd.case_id })?;
        case.assign_officer(cmd.officer_id);
        self.repo.save(&case).await
    }
    async fn decide(&self, cmd: DecideKybCommand) -> Result<(), PlatformError> {
        let mut case = self.repo.find_by_id(cmd.case_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "kyb_case".into(), id: cmd.case_id })?;
        case.decide(&cmd.decision, &cmd.reason);
        self.repo.save(&case).await
    }
    async fn get_case(&self, cmd: GetKybCaseCommand) -> Result<KybCase, PlatformError> {
        self.repo.find_by_id(cmd.case_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "kyb_case".into(), id: cmd.case_id })
    }
}
