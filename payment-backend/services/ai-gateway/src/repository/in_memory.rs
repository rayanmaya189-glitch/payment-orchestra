//! In-memory AI Gateway repository.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::AiGatewayRepository;

#[derive(Clone)]
pub struct InMemoryAiGatewayRepository {
    pub(super) audit_log: Arc<RwLock<Vec<GuardrailAuditEntry>>>,
    pub(super) quotas: Arc<RwLock<HashMap<Uuid, UsageQuota>>>,
    pub(super) circuit_breaker: Arc<RwLock<CircuitBreakerState>>,
}

impl Default for InMemoryAiGatewayRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAiGatewayRepository {
    pub fn new() -> Self {
        Self {
            audit_log: Arc::new(RwLock::new(Vec::new())),
            quotas: Arc::new(RwLock::new(HashMap::new())),
            circuit_breaker: Arc::new(RwLock::new(CircuitBreakerState::default())),
        }
    }
}

#[async_trait]
impl AiGatewayRepository for InMemoryAiGatewayRepository {
    async fn save_audit_entry(&self, entry: &GuardrailAuditEntry) -> Result<(), AiGatewayError> {
        self.audit_log.write().await.push(entry.clone());
        Ok(())
    }

    async fn list_audit_entries(&self, operator_id: Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError> {
        let log = self.audit_log.read().await;
        Ok(log.iter().filter(|e| e.operator_id == operator_id).cloned().collect())
    }

    async fn save_quota(&self, quota: &UsageQuota) -> Result<(), AiGatewayError> {
        self.quotas.write().await.insert(quota.operator_id, quota.clone());
        Ok(())
    }

    async fn load_quota(&self, operator_id: Uuid) -> Result<Option<UsageQuota>, AiGatewayError> {
        Ok(self.quotas.read().await.get(&operator_id).cloned())
    }

    async fn save_circuit_breaker(&self, state: &CircuitBreakerState) -> Result<(), AiGatewayError> {
        let mut cb = self.circuit_breaker.write().await;
        *cb = state.clone();
        Ok(())
    }

    async fn load_circuit_breaker(&self) -> Result<CircuitBreakerState, AiGatewayError> {
        Ok(self.circuit_breaker.read().await.clone())
    }
}
