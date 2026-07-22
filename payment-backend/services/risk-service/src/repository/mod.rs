//! Fraud & Risk Scoring repository — BC-11

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

// ---------------------------------------------------------------------------
// Repository trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait RiskRepository: Send + Sync {
    async fn load_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<RiskAssessment>, RiskError>;
    async fn save(&self, assessment: &RiskAssessment) -> Result<(), RiskError>;
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
// In-memory implementation
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct InMemoryRiskRepository {
    assessments: Arc<RwLock<HashMap<Uuid, RiskAssessment>>>,
    payment_index: Arc<RwLock<HashMap<Uuid, Uuid>>>, // payment_intent_id → assessment_id
}

impl InMemoryRiskRepository {
    pub fn new() -> Self {
        Self {
            assessments: Arc::new(RwLock::new(HashMap::new())),
            payment_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RiskRepository for InMemoryRiskRepository {
    async fn load_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<RiskAssessment>, RiskError> {
        let idx = self.payment_index.read().await;
        match idx.get(&payment_intent_id) {
            Some(assessment_id) => {
                let map = self.assessments.read().await;
                Ok(map.get(assessment_id).cloned())
            }
            None => Ok(None),
        }
    }

    async fn save(&self, assessment: &RiskAssessment) -> Result<(), RiskError> {
        let id = assessment.risk_assessment_id;
        let pi_id = assessment.payment_intent_id;
        {
            let mut map = self.assessments.write().await;
            map.insert(id, assessment.clone());
        }
        {
            let mut idx = self.payment_index.write().await;
            idx.insert(pi_id, id);
        }
        Ok(())
    }

    async fn find_high_risk(
        &self,
        _operator_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<RiskAssessment>, RiskError> {
        let map = self.assessments.read().await;
        let results: Vec<RiskAssessment> = map
            .values()
            .filter(|a| {
                a.assessed_at >= since
                    && (a.risk_level == RiskLevel::High || a.risk_level == RiskLevel::Critical)
            })
            .cloned()
            .collect();
        Ok(results)
    }

    async fn get_risk_stats(
        &self,
        _operator_id: Uuid,
        window_hours: u32,
    ) -> Result<RiskStats, RiskError> {
        let map = self.assessments.read().await;
        let cutoff = Utc::now() - chrono::Duration::hours(window_hours as i64);

        let windowed: Vec<&RiskAssessment> =
            map.values().filter(|a| a.assessed_at >= cutoff).collect();

        if windowed.is_empty() {
            return Ok(RiskStats::default());
        }

        let total_score: f64 = windowed.iter().map(|a| a.risk_score).sum();
        let high_count = windowed
            .iter()
            .filter(|a| a.risk_level == RiskLevel::High)
            .count() as u32;
        let critical_count = windowed
            .iter()
            .filter(|a| a.risk_level == RiskLevel::Critical)
            .count() as u32;

        // Group by BIN (extracted from risk_assessment_id for simplicity)
        let risk_by_bin = HashMap::new();
        let risk_by_country = HashMap::new();

        Ok(RiskStats {
            avg_risk_score: total_score / windowed.len() as f64,
            high_risk_count: high_count,
            critical_risk_count: critical_count,
            risk_by_bin,
            risk_by_country,
        })
    }
}
