//! Fraud & Risk Scoring command processing pipeline — BC-11

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Aggregates all dependencies for the risk service.
pub struct RiskPipeline {
    pub api: RiskApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryRiskRepository>>,
}

impl RiskPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryRiskRepository::new()));
        let repo_adapter = ArcRepoAdapter(repo.clone());

        let command_handler: Box<dyn CommandHandler> =
            Box::new(RiskCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(RiskQueryHandler::new(repo_adapter));
        let api = RiskApi::new(command_handler, query_handler);
        let event_bus: Box<dyn EventBus> = Box::new(NoopEventBus);

        Self {
            api,
            event_bus,
            repo,
        }
    }

    /// Override the default no-op event bus with a real NATS-backed bus.
    pub fn with_event_bus(mut self, event_bus: Box<dyn EventBus>) -> Self {
        self.event_bus = event_bus;
        self
    }
}

/// Adapter that wraps `Arc<RwLock<InMemoryRiskRepository>>` and
/// implements `RiskRepository` by delegating to the inner repo.
#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryRiskRepository>>);

#[async_trait::async_trait]
impl RiskRepository for ArcRepoAdapter {
    async fn load_by_payment_intent(
        &self,
        payment_intent_id: uuid::Uuid,
    ) -> Result<Option<RiskAssessment>, RiskError> {
        let repo = self.0.read().await;
        repo.load_by_payment_intent(payment_intent_id).await
    }

    async fn save(&self, assessment: &RiskAssessment) -> Result<(), RiskError> {
        let repo = self.0.write().await;
        repo.save(assessment).await
    }

    async fn find_high_risk(
        &self,
        operator_id: uuid::Uuid,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<RiskAssessment>, RiskError> {
        let repo = self.0.read().await;
        repo.find_high_risk(operator_id, since).await
    }

    async fn get_risk_stats(
        &self,
        operator_id: uuid::Uuid,
        window_hours: u32,
    ) -> Result<RiskStats, RiskError> {
        let repo = self.0.read().await;
        repo.get_risk_stats(operator_id, window_hours).await
    }
}
