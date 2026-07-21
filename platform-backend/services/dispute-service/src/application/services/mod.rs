use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::Dispute;
use crate::domain::rules::DisputeRepository;
use crate::domain::value_objects::DisputeDecision;
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};
use shared_types::{CurrencyCode, Money};

pub struct DisputeServiceImpl { repo: Box<dyn DisputeRepository>, db: DatabaseConnection }
impl DisputeServiceImpl {
    pub fn new(repo: Box<dyn DisputeRepository>, db: DatabaseConnection) -> Self { Self { repo, db } }

    fn check_abac(principal_id: Uuid, role: &str, action: &str, resource: &str) -> Result<(), PlatformError> {
        evaluate_policy(&AbacContext {
            principal_id, role: role.to_string(), action: action.to_string(),
            resource: resource.to_string(), resource_id: None, amount: None,
            ip_address: None, operator_id: None,
        })
    }
}

#[async_trait]
pub trait DisputeService: Send + Sync {
    async fn open(&self, cmd: OpenDisputeCommand) -> Result<Uuid, PlatformError>;
    async fn submit_evidence(&self, cmd: SubmitEvidenceCommand) -> Result<(), PlatformError>;
    async fn resolve(&self, cmd: ResolveDisputeCommand) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<Dispute, PlatformError>;
}

#[async_trait]
impl DisputeService for DisputeServiceImpl {
    async fn open(&self, cmd: OpenDisputeCommand) -> Result<Uuid, PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "create", "dispute")?;
        let amount = Money { amount_minor_units: cmd.amount_minor_units, currency: CurrencyCode::new(&cmd.currency).map_err(|_| PlatformError::Validation(platform_error::ValidationError::InvalidCurrencyCode))? };
        let d = Dispute::new(cmd.payment_intent_id, cmd.operator_id, cmd.reason, amount, cmd.acquirer_reference, cmd.connector_id);
        self.repo.save(&d).await?;
        Ok(d.dispute_id)
    }
    async fn submit_evidence(&self, cmd: SubmitEvidenceCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "dispute")?;
        let mut d = self.repo.find_by_id(cmd.dispute_id).await?.ok_or_else(|| PlatformError::NotFound { resource: "dispute".into(), id: cmd.dispute_id })?;
        d.submit_evidence(cmd.evidence);
        self.repo.save(&d).await
    }
    async fn resolve(&self, cmd: ResolveDisputeCommand) -> Result<(), PlatformError> {
        Self::check_abac(cmd.principal_id, &cmd.role, "update", "dispute")?;
        let mut d = self.repo.find_by_id(cmd.dispute_id).await?.ok_or_else(|| PlatformError::NotFound { resource: "dispute".into(), id: cmd.dispute_id })?;
        let decision = match cmd.decision.as_str() { "lost" => DisputeDecision::Lost, "expired" => DisputeDecision::Expired, _ => DisputeDecision::Won };
        d.resolve(decision, &cmd.reason);
        self.repo.save(&d).await
    }
    async fn get(&self, id: Uuid) -> Result<Dispute, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "dispute".into(), id })
    }
}
