//! AI Gateway repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait AiGatewayRepository: Send + Sync {
    async fn save_audit_entry(&self, entry: &GuardrailAuditEntry) -> Result<(), AiGatewayError>;
    async fn list_audit_entries(&self, operator_id: Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError>;
    async fn save_quota(&self, quota: &UsageQuota) -> Result<(), AiGatewayError>;
    async fn load_quota(&self, operator_id: Uuid) -> Result<Option<UsageQuota>, AiGatewayError>;
    async fn save_circuit_breaker(&self, state: &CircuitBreakerState) -> Result<(), AiGatewayError>;
    async fn load_circuit_breaker(&self) -> Result<CircuitBreakerState, AiGatewayError>;
}
