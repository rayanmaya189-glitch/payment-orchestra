//! Fraud & Risk Scoring query handlers — BC-11

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

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

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_assessment(&self, payment_intent_id: Uuid) -> Result<RiskAssessment, RiskError> {
        (**self).get_assessment(payment_intent_id).await
    }
    async fn find_high_risk(&self, operator_id: Uuid, since: DateTime<Utc>) -> Result<Vec<RiskAssessment>, RiskError> {
        (**self).find_high_risk(operator_id, since).await
    }
    async fn get_risk_stats(&self, operator_id: Uuid, window_hours: u32) -> Result<RiskStats, RiskError> {
        (**self).get_risk_stats(operator_id, window_hours).await
    }
}
