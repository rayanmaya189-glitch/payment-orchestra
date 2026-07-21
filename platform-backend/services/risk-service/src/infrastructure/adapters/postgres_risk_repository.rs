use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::domain::aggregates::RiskAssessment;
use crate::domain::rules::RiskAssessmentRepository;
use crate::domain::value_objects::{RiskDecision, RiskFactor, RiskFactorBreakdown};
use crate::infrastructure::entities::risk_assessment_entity;
use platform_error::PlatformError;

pub struct PostgresRiskRepository {
    db: DatabaseConnection,
}

impl PostgresRiskRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RiskAssessmentRepository for PostgresRiskRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskAssessment>, PlatformError> {
        let m = risk_assessment_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        Ok(m.map(|m| m.into()))
    }

    async fn save(&self, a: &RiskAssessment) -> Result<(), PlatformError> {
        let existing = risk_assessment_entity::Entity::find_by_id(a.assessment_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;

        let factors_json =
            serde_json::to_value(&a.factors).map_err(|e| PlatformError::Internal(format!("Serialize factors: {e}")))?;
        let breakdown_json = serde_json::to_value(&a.breakdown)
            .map_err(|e| PlatformError::Internal(format!("Serialize breakdown: {e}")))?;

        if let Some(_model) = existing {
            let am = risk_assessment_entity::ActiveModel {
                assessment_id: Set(a.assessment_id),
                payment_intent_id: Set(a.payment_intent_id),
                operator_id: Set(a.operator_id),
                score: Set(a.score),
                decision: Set(a.decision.as_str().to_string()),
                factors: Set(factors_json),
                breakdown: Set(breakdown_json),
                ip_address: Set(a.ip_address.clone()),
                user_agent: Set(a.user_agent.clone()),
                created_at: Set(a.created_at.into()),
            };
            am.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        } else {
            let am = risk_assessment_entity::ActiveModel {
                assessment_id: Set(a.assessment_id),
                payment_intent_id: Set(a.payment_intent_id),
                operator_id: Set(a.operator_id),
                score: Set(a.score),
                decision: Set(a.decision.as_str().to_string()),
                factors: Set(factors_json),
                breakdown: Set(breakdown_json),
                ip_address: Set(a.ip_address.clone()),
                user_agent: Set(a.user_agent.clone()),
                created_at: Set(a.created_at.into()),
            };
            am.insert(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        }
        Ok(())
    }
}

impl From<risk_assessment_entity::Model> for RiskAssessment {
    fn from(m: risk_assessment_entity::Model) -> Self {
        let factors: Vec<RiskFactor> = serde_json::from_value(m.factors).unwrap_or_default();
        let breakdown: RiskFactorBreakdown =
            serde_json::from_value(m.breakdown).unwrap_or_default();

        RiskAssessment {
            assessment_id: m.assessment_id,
            payment_intent_id: m.payment_intent_id,
            operator_id: m.operator_id,
            score: m.score,
            decision: RiskDecision::from_str(&m.decision),
            factors,
            breakdown,
            ip_address: m.ip_address,
            user_agent: m.user_agent,
            created_at: m.created_at.into(),
        }
    }
}
