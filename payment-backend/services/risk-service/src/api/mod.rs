//! Fraud & Risk Scoring API surface — BC-11

use crate::commands::*;
use crate::domain::{RiskAssessment, RiskError, RiskRule};
use crate::queries::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Public API facade for the risk service.
pub mod grpc;

pub struct RiskApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl RiskApi {
    pub fn new(
        command_handler: Box<dyn CommandHandler>,
        query_handler: Box<dyn QueryHandler>,
    ) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    pub async fn assess_risk(
        &self,
        cmd: AssessRiskCommand,
    ) -> Result<RiskAssessment, RiskError> {
        self.command_handler.assess_risk(cmd).await
    }

    pub async fn update_risk_rule(
        &self,
        cmd: UpdateRiskRuleCommand,
    ) -> Result<RiskRule, RiskError> {
        self.command_handler.update_risk_rule(cmd).await
    }

    pub async fn get_default_rules(&self) -> Vec<RiskRule> {
        self.command_handler.get_default_rules().await
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_assessment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<RiskAssessment, RiskError> {
        self.query_handler.get_assessment(payment_intent_id).await
    }

    pub async fn find_high_risk(
        &self,
        operator_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<RiskAssessment>, RiskError> {
        self.query_handler.find_high_risk(operator_id, since).await
    }

    pub async fn get_risk_stats(
        &self,
        operator_id: Uuid,
        window_hours: u32,
    ) -> Result<crate::domain::RiskStats, RiskError> {
        self.query_handler.get_risk_stats(operator_id, window_hours).await
    }
}
