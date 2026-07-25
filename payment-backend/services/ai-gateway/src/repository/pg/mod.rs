//! PostgreSQL repository for AI Gateway — hybrid PG + in-memory.
//!
//! The `ai_gateway_queries` entity stores raw query data. Domain types
//! `GuardrailAuditEntry`, `UsageQuota`, and `CircuitBreakerState` have no
//! corresponding entity tables, so they use in-memory storage.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::AiGatewayRepository;

#[derive(Clone)]
pub struct PostgresAiGatewayRepository {
    _db: sea_orm::DatabaseConnection,
    audit_entries: Arc<RwLock<HashMap<Uuid, GuardrailAuditEntry>>>,
    quotas: Arc<RwLock<HashMap<Uuid, UsageQuota>>>,
    circuit_breaker: Arc<RwLock<CircuitBreakerState>>,
}

impl PostgresAiGatewayRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self {
            _db: db,
            audit_entries: Arc::new(RwLock::new(HashMap::new())),
            quotas: Arc::new(RwLock::new(HashMap::new())),
            circuit_breaker: Arc::new(RwLock::new(CircuitBreakerState::default())),
        }
    }
}

#[async_trait]
impl AiGatewayRepository for PostgresAiGatewayRepository {
    async fn save_audit_entry(&self, entry: &GuardrailAuditEntry) -> Result<(), AiGatewayError> {
        self.audit_entries.write().await.insert(entry.audit_id, entry.clone());
        Ok(())
    }

    async fn list_audit_entries(&self, operator_id: Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError> {
        let entries = self.audit_entries.read().await;
        Ok(entries.values().filter(|e| e.operator_id == operator_id).cloned().collect())
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
