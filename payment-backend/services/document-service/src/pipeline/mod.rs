//! Document Management command processing pipeline — BC-13

use crate::api::*;
use crate::commands::*;
use crate::domain::*;
use crate::queries::*;
use crate::repository::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use platform_messaging::event_bus::{EventBus, NoopEventBus};

pub struct DocumentPipeline {
    pub api: DocumentApi,
    pub event_bus: Box<dyn EventBus>,
    pub repo: Arc<RwLock<InMemoryDocumentRepository>>,
}

impl DocumentPipeline {
    pub fn new() -> Self {
        let repo = Arc::new(RwLock::new(InMemoryDocumentRepository::new()));
        let repo_adapter = ArcRepoAdapter(repo.clone());

        let command_handler: Box<dyn CommandHandler> =
            Box::new(DocumentCommandHandler::new(repo_adapter.clone()));
        let query_handler: Box<dyn QueryHandler> =
            Box::new(DocumentQueryHandler::new(repo_adapter));
        let api = DocumentApi::new(command_handler, query_handler);
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

#[derive(Clone)]
pub struct ArcRepoAdapter(pub Arc<RwLock<InMemoryDocumentRepository>>);

#[async_trait::async_trait]
impl DocumentRepository for ArcRepoAdapter {
    async fn load(&self, id: uuid::Uuid) -> Result<Option<DocumentRecord>, DocumentError> {
        let repo = self.0.read().await;
        repo.load(id).await
    }

    async fn save(&self, record: &DocumentRecord) -> Result<(), DocumentError> {
        let repo = self.0.write().await;
        repo.save(record).await
    }

    async fn delete(&self, id: uuid::Uuid) -> Result<(), DocumentError> {
        let repo = self.0.write().await;
        repo.delete(id).await
    }

    async fn find_by_type(
        &self,
        operator_id: uuid::Uuid,
        category: &str,
    ) -> Result<Vec<DocumentRecord>, DocumentError> {
        let repo = self.0.read().await;
        repo.find_by_type(operator_id, category).await
    }

    async fn find_by_operator(&self, operator_id: uuid::Uuid) -> Result<Vec<DocumentRecord>, DocumentError> {
        let repo = self.0.read().await;
        repo.find_by_operator(operator_id).await
    }
}
