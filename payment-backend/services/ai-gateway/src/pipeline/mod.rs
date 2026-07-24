//! AI Gateway pipeline

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AiGatewayPipeline {
    pub api: AiGatewayApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryAiGatewayRepository>>,
}

use platform_messaging::event_bus::{EventBus, NoopEventBus};

impl Default for AiGatewayPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl AiGatewayPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryAiGatewayRepository::new()));
        let adapter = ArcRepoAdapter(repo.clone());
        let ch: Box<dyn CommandHandler> = Box::new(AiGatewayCommandHandler::new(adapter.clone()));
        let qh: Box<dyn QueryHandler> = Box::new(AiGatewayQueryHandler::new(adapter));
        let api = AiGatewayApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);
        Self { api, event_bus: eb, repo }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryAiGatewayRepository>>);

#[async_trait::async_trait]
impl AiGatewayRepository for ArcRepoAdapter {
    async fn save_audit_entry(&self, entry: &GuardrailAuditEntry) -> Result<(), AiGatewayError> {
        self.0.write().await.save_audit_entry(entry).await
    }
    async fn list_audit_entries(&self, oid: uuid::Uuid) -> Result<Vec<GuardrailAuditEntry>, AiGatewayError> {
        self.0.read().await.list_audit_entries(oid).await
    }
    async fn save_quota(&self, quota: &UsageQuota) -> Result<(), AiGatewayError> {
        self.0.write().await.save_quota(quota).await
    }
    async fn load_quota(&self, oid: uuid::Uuid) -> Result<Option<UsageQuota>, AiGatewayError> {
        self.0.read().await.load_quota(oid).await
    }
    async fn save_circuit_breaker(&self, state: &CircuitBreakerState) -> Result<(), AiGatewayError> {
        self.0.write().await.save_circuit_breaker(state).await
    }
    async fn load_circuit_breaker(&self) -> Result<CircuitBreakerState, AiGatewayError> {
        self.0.read().await.load_circuit_breaker().await
    }
}
