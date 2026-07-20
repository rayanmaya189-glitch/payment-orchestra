use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use crate::domain::aggregates::RiskAssessment;
use crate::domain::value_objects::RiskDecision;
use crate::domain::rules::RiskAssessmentRepository;
use crate::infrastructure::entities::risk_assessment_entity;
use platform_error::PlatformError;

pub struct PostgresRiskRepository { db: DatabaseConnection }
impl PostgresRiskRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl RiskAssessmentRepository for PostgresRiskRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RiskAssessment>, PlatformError> {
        let m = risk_assessment_entity::Entity::find_by_id(id).one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn save(&self, a: &RiskAssessment) -> Result<(), PlatformError> {
        let existing = risk_assessment_entity::Entity::find_by_id(a.assessment_id).one(&self.db).await
            .map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        if let Some(model) = existing {
            let mut am = risk_assessment_entity::ActiveModel::from(model);
            am.score = Set(a.score);
            am.decision = Set(a.decision.as_str().to_string());
            am.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        } else {
            let am = risk_assessment_entity::ActiveModel {
                assessment_id: Set(a.assessment_id), payment_intent_id: Set(a.payment_intent_id),
                operator_id: Set(a.operator_id), score: Set(a.score),
                decision: Set(a.decision.as_str().to_string()),
                factors: Set(serde_json::to_value(&a.factors).unwrap()),
                ip_address: Set(a.ip_address.clone()), user_agent: Set(a.user_agent.clone()),
                velocity_score: Set(a.velocity_score), geo_score: Set(a.geo_score),
                behavior_score: Set(a.behavior_score),
                is_whitelisted: Set(a.is_whitelisted), is_blacklisted: Set(a.is_blacklisted),
                created_at: Set(a.created_at.into()),
            };
            am.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB error: {e}")))?;
        }
        Ok(())
    }
}

impl From<risk_assessment_entity::Model> for RiskAssessment {
    fn from(m: risk_assessment_entity::Model) -> Self {
        RiskAssessment {
            assessment_id: m.assessment_id, payment_intent_id: m.payment_intent_id,
            operator_id: m.operator_id, score: m.score,
            decision: RiskDecision::from_str(&m.decision),
            factors: Vec::new(), ip_address: m.ip_address, user_agent: m.user_agent,
            velocity_score: m.velocity_score, geo_score: m.geo_score,
            behavior_score: m.behavior_score,
            is_whitelisted: m.is_whitelisted, is_blacklisted: m.is_blacklisted,
            created_at: m.created_at.into(),
        }
    }
}
