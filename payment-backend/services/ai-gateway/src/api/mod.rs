//! AI Gateway public API

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

pub struct AiGatewayApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl AiGatewayApi {
    pub fn new(ch: Box<dyn CommandHandler>, qh: Box<dyn QueryHandler>) -> Self {
        Self { command_handler: ch, query_handler: qh }
    }

    pub async fn process_query(&self, cmd: ProcessAiQuery) -> Result<GuardrailResult, AiGatewayError> {
        self.command_handler.process_query(cmd).await
    }
    pub async fn record_model_failure(&self) -> Result<(), AiGatewayError> {
        self.command_handler.record_model_failure().await
    }
    pub async fn record_model_success(&self) -> Result<(), AiGatewayError> {
        self.command_handler.record_model_success().await
    }
    pub async fn reset_quota(&self, cmd: ResetQuota) -> Result<(), AiGatewayError> {
        self.command_handler.reset_quota(cmd).await
    }
    pub async fn get_circuit_breaker_state(&self) -> Result<CircuitBreakerState, AiGatewayError> {
        self.query_handler.get_circuit_breaker_state().await
    }
    pub async fn get_quota(&self, operator_id: Uuid) -> Result<UsageQuota, AiGatewayError> {
        self.query_handler.get_quota(operator_id).await
    }
    pub async fn get_audit_log(&self, operator_id: Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError> {
        self.query_handler.get_audit_log(operator_id).await
    }
    pub async fn get_health_status(&self) -> GuardrailResult {
        self.query_handler.get_health_status().await
    }
}
