//! Dispute Management repository trait — BC-10

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait DisputeRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<ChargebackCase>, DisputeError>;
    async fn save(&self, case: &ChargebackCase) -> Result<(), DisputeError>;
    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError>;
    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, DisputeError>;
}
