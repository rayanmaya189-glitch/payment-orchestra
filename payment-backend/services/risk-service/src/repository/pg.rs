//! PostgreSQL-backed RiskRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::RiskRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as RiskAssessmentActiveModel,
    Column as RiskAssessmentColumn,
    Entity as RiskAssessmentEntity,
    Model as RiskAssessmentModel,
};

pub struct PostgresRiskRepository {
    pub db: DatabaseConnection,
}

impl PostgresRiskRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RiskRepository for PostgresRiskRepository {
    async fn load_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<RiskAssessment>, RiskError> {
        let result = RiskAssessmentEntity::find()
            .filter(RiskAssessmentColumn::PaymentIntentId.eq(payment_intent_id))
            .one(&self.db)
            .await
            .map_err(|e| RiskError::NotFound(payment_intent_id))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, assessment: &RiskAssessment) -> Result<(), RiskError> {
        let model = domain_to_model(assessment);
        let exists = RiskAssessmentEntity::find_by_id(assessment.risk_assessment_id)
            .one(&self.db)
            .await
            .map_err(|e| RiskError::NotFound(assessment.risk_assessment_id))?
            .is_some();

        if exists {
            RiskAssessmentEntity::update(RiskAssessmentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| RiskError::NotFound(assessment.risk_assessment_id))?;
        } else {
            RiskAssessmentEntity::insert(RiskAssessmentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| RiskError::NotFound(assessment.risk_assessment_id))?;
        }
        Ok(())
    }

    async fn find_high_risk(&self, operator_id: Uuid, since: DateTime<Utc>) -> Result<Vec<RiskAssessment>, RiskError> {
        // Risk assessments don't have operator_id directly - use payment_intents join or store operator_id
        let models = RiskAssessmentEntity::find()
            .filter(RiskAssessmentColumn::RiskLevel.is_in(vec!["high", "critical"]))
            .filter(RiskAssessmentColumn::AssessedAt.gte(since))
            .all(&self.db)
            .await
            .map_err(|e| RiskError::NotFound(operator_id))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn get_risk_stats(&self, _operator_id: Uuid, _window_hours: u32) -> Result<RiskStats, RiskError> {
        // Return default stats - will be enhanced in production
        Ok(RiskStats::default())
    }
}

fn domain_to_model(a: &RiskAssessment) -> RiskAssessmentModel {
    let risk_factors = serde_json::to_value(&a.risk_factors).unwrap_or_default();
    RiskAssessmentModel {
        risk_assessment_id: a.risk_assessment_id,
        payment_intent_id: a.payment_intent_id,
        risk_score: a.risk_score,
        risk_level: a.risk_level.to_string(),
        risk_factors,
        rule_version: a.rule_version.clone(),
        assessed_at: a.assessed_at,
    }
}

fn model_to_domain(m: RiskAssessmentModel) -> Result<RiskAssessment, RiskError> {
    let risk_level = RiskLevel::from_score(m.risk_score);
    let risk_factors: Vec<String> = serde_json::from_value(m.risk_factors)
        .unwrap_or_default();

    Ok(RiskAssessment {
        risk_assessment_id: m.risk_assessment_id,
        payment_intent_id: m.payment_intent_id,
        risk_score: m.risk_score,
        risk_level,
        risk_factors,
        rule_version: m.rule_version,
        assessed_at: m.assessed_at,
    })
}
