//! Risk Scoring repository trait — BC-11

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait RiskRepository: Send + Sync {
    async fn load_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<RiskAssessment>, RiskError>;
    async fn save(&self, assessment: &RiskAssessment) -> Result<(), RiskError>;
    async fn find_high_risk(&self, operator_id: Uuid, since: DateTime<Utc>) -> Result<Vec<RiskAssessment>, RiskError>;
    async fn get_risk_stats(&self, operator_id: Uuid, window_hours: u32) -> Result<RiskStats, RiskError>;
}
