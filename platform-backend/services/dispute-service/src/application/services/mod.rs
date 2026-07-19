use async_trait::async_trait; use uuid::Uuid;
use crate::domain::aggregates::Dispute;
use crate::domain::value_objects::{DisputeReason, DisputeStatus};
use platform_error::PlatformError;

#[async_trait]
pub trait DisputeService: Send + Sync {
    async fn open_dispute(&self, cmd: OpenDisputeCommand) -> Result<DisputeResponse, PlatformError>;
    async fn get_dispute(&self, id: Uuid) -> Result<DisputeResponse, PlatformError>;
    async fn submit_evidence(&self, id: Uuid, evidence: String) -> Result<(), PlatformError>;
    async fn resolve_dispute(&self, id: Uuid) -> Result<(), PlatformError>;
}

pub struct DisputeServiceImpl { db: sea_orm::DatabaseConnection }
impl DisputeServiceImpl { pub fn new(db: sea_orm::DatabaseConnection) -> Self { Self { db } } }

pub struct OpenDisputeCommand { pub operator_id: Uuid, pub payment_intent_id: Uuid, pub reason: String, pub amount: shared_types::Money }
#[derive(Debug, Clone)]
pub struct DisputeResponse { pub dispute_id: Uuid, pub status: String, pub reason: String, pub amount: i64, pub currency: String }

#[async_trait]
impl DisputeService for DisputeServiceImpl {
    async fn open_dispute(&self, cmd: OpenDisputeCommand) -> Result<DisputeResponse, PlatformError> {
        let reason = match cmd.reason.as_str() { "fraudulent" => DisputeReason::Fraudulent, "not_received" => DisputeReason::NotReceived, "not_as_described" => DisputeReason::NotAsDescribed, "duplicate" => DisputeReason::Duplicate, _ => DisputeReason::Other(cmd.reason) };
        let dispute = Dispute::new(cmd.operator_id, cmd.payment_intent_id, reason, cmd.amount);
        Ok(dispute_to_response(&dispute))
    }
    async fn get_dispute(&self, id: Uuid) -> Result<DisputeResponse, PlatformError> { Err(PlatformError::NotFound { resource: "Dispute".into(), id }) }
    async fn submit_evidence(&self, _id: Uuid, _evidence: String) -> Result<(), PlatformError> { Ok(()) }
    async fn resolve_dispute(&self, _id: Uuid) -> Result<(), PlatformError> { Ok(()) }
}

fn dispute_to_response(d: &Dispute) -> DisputeResponse { DisputeResponse { dispute_id: d.dispute_id, status: d.status.as_str().to_string(), reason: d.reason.as_str().to_string(), amount: d.amount.amount_minor_units, currency: d.amount.currency.0.clone() } }
