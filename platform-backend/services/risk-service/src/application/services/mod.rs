use async_trait::async_trait; use uuid::Uuid;
use crate::domain::aggregates::RiskAssessment;
use crate::domain::value_objects::RiskFactor;
use platform_error::PlatformError;

#[async_trait]
pub trait RiskService: Send + Sync {
    async fn assess_risk(&self, cmd: AssessRiskCommand) -> Result<RiskAssessmentResponse, PlatformError>;
    async fn get_assessment(&self, id: Uuid) -> Result<RiskAssessmentResponse, PlatformError>;
}

pub struct RiskServiceImpl { db: sea_orm::DatabaseConnection }
impl RiskServiceImpl { pub fn new(db: sea_orm::DatabaseConnection) -> Self { Self { db } } }

pub struct AssessRiskCommand { pub operator_id: Uuid, pub payment_intent_id: Uuid, pub amount: shared_types::Money }
#[derive(Debug, Clone)]
pub struct RiskAssessmentResponse { pub assessment_id: Uuid, pub score: f64, pub decision: String, pub factors: Vec<RiskFactor> }

#[async_trait]
impl RiskService for RiskServiceImpl {
    async fn assess_risk(&self, cmd: AssessRiskCommand) -> Result<RiskAssessmentResponse, PlatformError> {
        let mut assessment = RiskAssessment::new(cmd.operator_id, cmd.payment_intent_id, cmd.amount);
        // Add default risk factors
        assessment.factors.push(RiskFactor { factor: "amount_check".into(), score: 0.9, weight: 1.0, description: "Amount within normal range".into() });
        assessment.factors.push(RiskFactor { factor: "velocity_check".into(), score: 0.85, weight: 0.5, description: "Normal transaction velocity".into() });
        assessment.evaluate();
        Ok(assessment_to_response(&assessment))
    }
    async fn get_assessment(&self, id: Uuid) -> Result<RiskAssessmentResponse, PlatformError> { Err(PlatformError::NotFound { resource: "RiskAssessment".into(), id }) }
}

fn assessment_to_response(a: &RiskAssessment) -> RiskAssessmentResponse { RiskAssessmentResponse { assessment_id: a.assessment_id, score: a.score, decision: a.decision.as_str().to_string(), factors: a.factors.clone() } }
