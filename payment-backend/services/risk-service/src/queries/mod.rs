//! Fraud & Risk Scoring query handlers — BC-11

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

// ---------------------------------------------------------------------------
// Query handler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_assessment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<RiskAssessment, RiskError>;
    async fn find_high_risk(
        &self,
        operator_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<RiskAssessment>, RiskError>;
    async fn get_risk_stats(
        &self,
        operator_id: Uuid,
        window_hours: u32,
    ) -> Result<RiskStats, RiskError>;
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

pub struct RiskQueryHandler<R: RiskRepository> {
    repo: R,
}

impl<R: RiskRepository> RiskQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: RiskRepository + Send + Sync> QueryHandler for RiskQueryHandler<R> {
    async fn get_assessment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<RiskAssessment, RiskError> {
        self.repo
            .load_by_payment_intent(payment_intent_id)
            .await?
            .ok_or(RiskError::NotFound(payment_intent_id))
    }

    async fn find_high_risk(
        &self,
        operator_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<RiskAssessment>, RiskError> {
        self.repo.find_high_risk(operator_id, since).await
    }

    async fn get_risk_stats(
        &self,
        operator_id: Uuid,
        window_hours: u32,
    ) -> Result<RiskStats, RiskError> {
        self.repo.get_risk_stats(operator_id, window_hours).await
    }
}
