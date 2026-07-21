use async_trait::async_trait;
use uuid::Uuid;
use crate::domain::aggregates::Document;
use crate::domain::rules::DocumentRepository;
use platform_error::PlatformError;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory repository for testing.
pub struct NoopDocumentRepository {
    store: RwLock<HashMap<Uuid, Document>>,
}

impl NoopDocumentRepository {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl DocumentRepository for NoopDocumentRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Document>, PlatformError> {
        Ok(self.store.read().unwrap().get(&id).cloned())
    }

    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Document>, PlatformError> {
        let store = self.store.read().unwrap();
        let results: Vec<Document> = store
            .values()
            .filter(|d| d.operator_id == operator_id)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn save(&self, document: &Document) -> Result<(), PlatformError> {
        self.store
            .write()
            .unwrap()
            .insert(document.document_id, document.clone());
        Ok(())
    }

    async fn count_by_operator_and_type(
        &self,
        operator_id: Uuid,
        document_type: &str,
    ) -> Result<i64, PlatformError> {
        let store = self.store.read().unwrap();
        let count = store
            .values()
            .filter(|d| d.operator_id == operator_id && d.document_type == document_type)
            .count() as i64;
        Ok(count)
    }
}
