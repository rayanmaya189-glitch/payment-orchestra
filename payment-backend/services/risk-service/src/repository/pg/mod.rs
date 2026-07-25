//! PostgreSQL repository for Risk Service — BC-11
//!
//! Persists [`RiskAssessment`] to the `risk_assessments` table.
//! Uses PG aggregation for `get_risk_stats`.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::risk_assessment::{self, Entity as RiskAssessmentEntity, Column as RiskAssessmentColumn};
use crate::repository::RiskRepository;

#[derive(Clone)]
pub struct PostgresRiskRepository {
    db: sea_orm::DatabaseConnection,
}

impl PostgresRiskRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RiskRepository for PostgresRiskRepository {
    async fn load_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<RiskAssessment>, RiskError> {
        let result = RiskAssessmentEntity::find()
            .filter(RiskAssessmentColumn::PaymentIntentId.eq(payment_intent_id))
            .order_by_desc(RiskAssessmentColumn::AssessedAt)
            .one(&self.db)
            .await
            .map_err(|e| RiskError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, assessment: &RiskAssessment) -> Result<(), RiskError> {
        let model = domain_to_model(assessment);

        risk_assessment::Entity::insert(model.clone())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(risk_assessment::Column::RiskAssessmentId)
                    .update_columns([
                        risk_assessment::Column::PaymentIntentId,
                        risk_assessment::Column::RiskScore,
                        risk_assessment::Column::RiskLevel,
                        risk_assessment::Column::RiskFactors,
                        risk_assessment::Column::RuleVersion,
                        risk_assessment::Column::AssessedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| RiskError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_high_risk(&self, _operator_id: Uuid, since: DateTime<Utc>) -> Result<Vec<RiskAssessment>, RiskError> {
        let results = RiskAssessmentEntity::find()
            .filter(RiskAssessmentColumn::AssessedAt.gte(since))
            .filter(
                sea_orm::Condition::any()
                    .add(RiskAssessmentColumn::RiskLevel.eq("high"))
                    .add(RiskAssessmentColumn::RiskLevel.eq("critical")),
            )
            .order_by_desc(RiskAssessmentColumn::AssessedAt)
            .all(&self.db)
            .await
            .map_err(|e| RiskError::DatabaseError(e.to_string()))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn get_risk_stats(&self, _operator_id: Uuid, window_hours: u32) -> Result<RiskStats, RiskError> {
        let cutoff = Utc::now() - Duration::hours(window_hours as i64);

        let results = RiskAssessmentEntity::find()
            .filter(RiskAssessmentColumn::AssessedAt.gte(cutoff))
            .all(&self.db)
            .await
            .map_err(|e| RiskError::DatabaseError(e.to_string()))?;

        if results.is_empty() {
            return Ok(RiskStats::default());
        }

        let assessments: Vec<RiskAssessment> = results
            .into_iter()
            .map(model_to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        let total_score: f64 = assessments.iter().map(|a| a.risk_score).sum();
        let high_count = assessments.iter().filter(|a| a.risk_level == RiskLevel::High).count() as u32;
        let critical_count = assessments.iter().filter(|a| a.risk_level == RiskLevel::Critical).count() as u32;

        Ok(RiskStats {
            avg_risk_score: total_score / assessments.len() as f64,
            high_risk_count: high_count,
            critical_risk_count: critical_count,
            risk_by_bin: std::collections::HashMap::new(),
            risk_by_country: std::collections::HashMap::new(),
        })
    }
}

// ─── Domain <-> Entity conversion helpers ─────────────────────────────────────

fn domain_to_model(assessment: &RiskAssessment) -> risk_assessment::ActiveModel {
    risk_assessment::ActiveModel {
        risk_assessment_id: sea_orm::ActiveValue::Set(assessment.risk_assessment_id),
        payment_intent_id: sea_orm::ActiveValue::Set(assessment.payment_intent_id),
        risk_score: sea_orm::ActiveValue::Set(assessment.risk_score),
        risk_level: sea_orm::ActiveValue::Set(assessment.risk_level.to_string()),
        risk_factors: sea_orm::ActiveValue::Set(serde_json::to_value(&assessment.risk_factors).unwrap_or_default()),
        rule_version: sea_orm::ActiveValue::Set(assessment.rule_version.clone()),
        assessed_at: sea_orm::ActiveValue::Set(assessment.assessed_at),
    }
}

fn model_to_domain(m: risk_assessment::Model) -> Result<RiskAssessment, RiskError> {
    let risk_level = match m.risk_level.as_str() {
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" => RiskLevel::Critical,
        other => return Err(RiskError::DatabaseError(format!("Invalid risk_level: {other}"))),
    };

    let risk_factors: Vec<String> = serde_json::from_value(m.risk_factors)
        .map_err(|e| RiskError::DatabaseError(format!("Invalid risk_factors JSON: {e}")))?;

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

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_assessment() -> RiskAssessment {
        let mut a = RiskAssessment::new(Uuid::now_v7());
        a.risk_score = 0.85;
        a.risk_level = RiskLevel::High;
        a.risk_factors = vec!["High amount".into(), "Geo mismatch".into()];
        a
    }

    #[tokio::test]
    async fn test_domain_to_model() {
        let a = sample_assessment();
        let model = domain_to_model(&a);

        assert_eq!(model.risk_assessment_id.unwrap(), a.risk_assessment_id);
        assert_eq!(model.payment_intent_id.unwrap(), a.payment_intent_id);
        assert!((model.risk_score.unwrap() - 0.85).abs() < f64::EPSILON);
        assert_eq!(model.risk_level.unwrap(), "high");
        assert_eq!(model.rule_version.unwrap(), "1.0");
    }

    #[tokio::test]
    async fn test_model_to_domain() {
        let a = sample_assessment();
        let entity = risk_assessment::Model {
            risk_assessment_id: a.risk_assessment_id,
            payment_intent_id: a.payment_intent_id,
            risk_score: 0.75,
            risk_level: "medium".into(),
            risk_factors: serde_json::to_value(vec!["Test factor".to_string()]).unwrap(),
            rule_version: "2.0".into(),
            assessed_at: a.assessed_at,
        };

        let domain = model_to_domain(entity).unwrap();
        assert_eq!(domain.risk_level, RiskLevel::Medium);
        assert!((domain.risk_score - 0.75).abs() < f64::EPSILON);
        assert_eq!(domain.risk_factors, vec!["Test factor"]);
        assert_eq!(domain.rule_version, "2.0");
    }

    #[tokio::test]
    async fn test_invalid_risk_level() {
        let a = sample_assessment();
        let entity = risk_assessment::Model {
            risk_assessment_id: a.risk_assessment_id,
            payment_intent_id: a.payment_intent_id,
            risk_score: 0.0,
            risk_level: "nonexistent".into(),
            risk_factors: serde_json::Value::Array(vec![]),
            rule_version: "1.0".into(),
            assessed_at: a.assessed_at,
        };

        let result = model_to_domain(entity);
        assert!(result.is_err());
        assert!(matches!(result, Err(RiskError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn test_invalid_risk_factors_json() {
        let a = sample_assessment();
        let entity = risk_assessment::Model {
            risk_assessment_id: a.risk_assessment_id,
            payment_intent_id: a.payment_intent_id,
            risk_score: 0.0,
            risk_level: "low".into(),
            risk_factors: serde_json::Value::String("not_an_array".into()),
            rule_version: "1.0".into(),
            assessed_at: a.assessed_at,
        };

        let result = model_to_domain(entity);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_all_risk_levels_roundtrip() {
        let a = sample_assessment();
        for (level_str, level) in [
            ("low", RiskLevel::Low),
            ("medium", RiskLevel::Medium),
            ("high", RiskLevel::High),
            ("critical", RiskLevel::Critical),
        ] {
            let entity = risk_assessment::Model {
                risk_assessment_id: a.risk_assessment_id,
                payment_intent_id: a.payment_intent_id,
                risk_score: 0.5,
                risk_level: level_str.into(),
                risk_factors: serde_json::Value::Array(vec![]),
                rule_version: "1.0".into(),
                assessed_at: a.assessed_at,
            };
            let domain = model_to_domain(entity).unwrap();
            assert_eq!(domain.risk_level, level);
        }
    }

    #[cfg(feature = "integration_test")]
    #[tokio::test]
    async fn test_pg_roundtrip() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for integration tests");
        let db = sea_orm::Database::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        let repo = PostgresRiskRepository::new(db.clone());
        let assessment = sample_assessment();

        // Save
        repo.save(&assessment).await.unwrap();

        // Load by payment intent
        let loaded = repo.load_by_payment_intent(assessment.payment_intent_id)
            .await
            .unwrap()
            .expect("Assessment should exist");
        assert_eq!(loaded.risk_assessment_id, assessment.risk_assessment_id);
        assert_eq!(loaded.risk_level, assessment.risk_level);

        // Clean up
        risk_assessment::Entity::delete_by_id(assessment.risk_assessment_id)
            .exec(&db)
            .await
            .unwrap();
    }
}
