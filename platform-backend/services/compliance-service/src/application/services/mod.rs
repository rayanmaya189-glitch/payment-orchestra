use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::KybCase;
use crate::domain::rules::KybCaseRepository;
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};

pub struct ComplianceServiceImpl { repo: Box<dyn KybCaseRepository>, db: DatabaseConnection }
impl ComplianceServiceImpl {
    pub fn new(repo: Box<dyn KybCaseRepository>, db: DatabaseConnection) -> Self { Self { repo, db } }

    fn check_abac(principal_id: Uuid, role: &str, action: &str, resource: &str) -> Result<(), PlatformError> {
        evaluate_policy(&AbacContext {
            principal_id, role: role.to_string(), action: action.to_string(),
            resource: resource.to_string(), resource_id: None, amount: None,
            ip_address: None, operator_id: None,
        })
    }
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
        Self::check_abac(cmd.principal_id, &cmd.role, "create", "kyb_case")?;
        let mut case = KybCase::new(cmd.operator_id);
        self.repo.save(&case).await?;
        Ok(case.case_id)
    }
    async fn assign_officer(&self, cmd: AssignOfficerCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "kyb_case")?;
        let mut case = self.repo.find_by_id(cmd.case_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "kyb_case".into(), id: cmd.case_id })?;
        case.assign_officer(cmd.officer_id);
        self.repo.save(&case).await
    }
    async fn decide(&self, cmd: DecideKybCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "kyb_case")?;
        let mut case = self.repo.find_by_id(cmd.case_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "kyb_case".into(), id: cmd.case_id })?;
        case.decide(&cmd.decision, &cmd.reason);
        self.repo.save(&case).await
    }
    async fn get_case(&self, cmd: GetKybCaseCommand) -> Result<KybCase, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "read", "kyb_case")?;
        self.repo.find_by_id(cmd.case_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "kyb_case".into(), id: cmd.case_id })
    }
}
