//! Document Management repository — BC-13

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<DocumentRecord>, DocumentError>;
    async fn save(&self, record: &DocumentRecord) -> Result<(), DocumentError>;
    async fn delete(&self, id: Uuid) -> Result<(), DocumentError>;
    async fn find_by_type(
        &self,
        operator_id: Uuid,
        category: &str,
    ) -> Result<Vec<DocumentRecord>, DocumentError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError>;
}

#[derive(Clone)]
pub struct InMemoryDocumentRepository {
    records: Arc<RwLock<HashMap<Uuid, DocumentRecord>>>,
}

impl InMemoryDocumentRepository {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DocumentRepository for InMemoryDocumentRepository {
    async fn load(&self, id: Uuid) -> Result<Option<DocumentRecord>, DocumentError> {
        let map = self.records.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, record: &DocumentRecord) -> Result<(), DocumentError> {
        let mut map = self.records.write().await;
        map.insert(record.document_id, record.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DocumentError> {
        let mut map = self.records.write().await;
        map.remove(&id);
        Ok(())
    }

    async fn find_by_type(
        &self,
        operator_id: Uuid,
        category: &str,
    ) -> Result<Vec<DocumentRecord>, DocumentError> {
        let map = self.records.read().await;
        let results: Vec<DocumentRecord> = map
            .values()
            .filter(|d| d.operator_id == operator_id && d.category.to_string() == category)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError> {
        let map = self.records.read().await;
        let results: Vec<DocumentRecord> = map
            .values()
            .filter(|d| d.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }
}
