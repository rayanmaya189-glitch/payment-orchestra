//! API Gateway pipeline

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::events::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GatewayPipeline {
    pub api: GatewayApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryGatewayRepository>>,
}

use platform_messaging::event_bus::{EventBus, NoopEventBus};

impl GatewayPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryGatewayRepository::new()));
        let adapter = ArcRepoAdapter(repo.clone());
        let ch: Box<dyn CommandHandler> = Box::new(GatewayCommandHandler::new(adapter.clone()));
        let qh: Box<dyn QueryHandler> = Box::new(GatewayQueryHandler::new(adapter));
        let api = GatewayApi::new(ch, qh);
        let eb: Box<dyn EventBus> = Box::new(NoopEventBus);
        Self { api, event_bus: eb, repo }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryGatewayRepository>>);

#[async_trait::async_trait]
impl GatewayRepository for ArcRepoAdapter {
    async fn get_route(&self, method: &str, path: &str) -> Result<Option<RouteDefinition>, GatewayError> {
        self.0.read().await.get_route(method, path).await
    }
    async fn list_routes(&self) -> Result<Vec<RouteDefinition>, GatewayError> {
        self.0.read().await.list_routes().await
    }
    async fn check_rate_limit(&self, key: &str, config: &RateLimitConfig) -> Result<bool, GatewayError> {
        self.0.write().await.check_rate_limit(key, config).await
    }
    async fn validate_api_key(&self, key: &str) -> Result<AuthResult, GatewayError> {
        self.0.read().await.validate_api_key(key).await
    }
    async fn log_request(&self, req: &ProcessedRequest) -> Result<(), GatewayError> {
        self.0.write().await.log_request(req).await
    }
    async fn get_request_log(&self, id: uuid::Uuid) -> Result<Option<ProcessedRequest>, GatewayError> {
        self.0.read().await.get_request_log(id).await
    }
}
