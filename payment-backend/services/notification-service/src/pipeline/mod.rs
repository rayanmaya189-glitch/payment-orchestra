//! Notification Service command processing pipeline — BC-14

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::events::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

pub struct NotificationPipeline {
    pub api: NotificationApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryNotificationRepository>>,
}

impl NotificationPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryNotificationRepository::new()));
        let repo_adapter = ArcRepoAdapter(repo.clone());

        let command_handler: Box<dyn CommandHandler> =
            Box::new(NotificationCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(NotificationQueryHandler::new(repo_adapter));
        let api = NotificationApi::new(command_handler, query_handler);
        let event_bus: Box<dyn EventBus> = Box::new(NoopEventBus);

        Self {
            api,
            event_bus,
            repo,
        }
    }
}

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryNotificationRepository>>);

#[async_trait::async_trait]
impl NotificationRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<NotificationRequest>, NotificationError> {
        let repo = self.0.read().await;
        repo.load(id).await
    }

    async fn save(&self, request: &NotificationRequest) -> Result<(), NotificationError> {
        let repo = self.0.write().await;
        repo.save(request).await
    }

    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let repo = self.0.read().await;
        repo.find_pending().await
    }

    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let repo = self.0.read().await;
        repo.find_dead_letter().await
    }

    async fn find_by_operator(&self, operator_id: uuid::Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        let repo = self.0.read().await;
        repo.find_by_operator(operator_id).await
    }
}
