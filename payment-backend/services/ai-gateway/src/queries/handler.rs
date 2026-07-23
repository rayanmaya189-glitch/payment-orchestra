//! AI Gateway query handlers

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_circuit_breaker_state(&self) -> Result<CircuitBreakerState, AiGatewayError>;
    async fn get_quota(&self, operator_id: Uuid) -> Result<UsageQuota, AiGatewayError>;
    async fn get_audit_log(&self, operator_id: Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError>;
    async fn get_health_status(&self) -> GuardrailResult;
    async fn list_injection_patterns(&self) -> Vec<String>;
}

pub struct AiGatewayQueryHandler<R: AiGatewayRepository> {
    repo: R,
}

impl<R: AiGatewayRepository> AiGatewayQueryHandler<R> {
    pub fn new(repo: R) -> Self { Self { repo } }
}

#[async_trait]
impl<R: AiGatewayRepository + Send + Sync> QueryHandler for AiGatewayQueryHandler<R> {
    async fn get_circuit_breaker_state(&self) -> Result<CircuitBreakerState, AiGatewayError> {
        self.repo.load_circuit_breaker().await
    }

    async fn get_quota(&self, operator_id: Uuid) -> Result<UsageQuota, AiGatewayError> {
        self.repo.load_quota(operator_id).await?
            .ok_or(AiGatewayError::RateLimited("No quota found, creating default".into()))
    }

    async fn get_audit_log(&self, operator_id: Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError> {
        self.repo.list_audit_entries(operator_id).await
    }

    async fn get_health_status(&self) -> GuardrailResult {
        GuardrailResult {
            query_id: Uuid::now_v7(),
            blocked: false,
            block_reason: None,
            screening_score: 0.0,
            model_route: ModelRoute::TextOnly,
            citations_valid: true,
            quota_available: true,
            quota_remaining: 100,
            circuit_open: false,
        }
    }

    async fn list_injection_patterns(&self) -> Vec<String> {
        INJECTION_PATTERNS.iter().map(|s| s.to_string()).collect()
    }
}
