//! Document Management query handlers — BC-13

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_document(&self, id: Uuid) -> Result<DocumentRecord, DocumentError>;
    async fn find_by_type(&self, operator_id: Uuid, category: &str) -> Result<Vec<DocumentRecord>, DocumentError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError>;
}

pub struct DocumentQueryHandler<R: DocumentRepository> {
    repo: R,
}

impl<R: DocumentRepository> DocumentQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: DocumentRepository + Send + Sync> QueryHandler for DocumentQueryHandler<R> {
    async fn get_document(&self, id: Uuid) -> Result<DocumentRecord, DocumentError> {
        self.repo.load(id).await?.ok_or(DocumentError::NotFound(id))
    }

    async fn find_by_type(&self, operator_id: Uuid, category: &str) -> Result<Vec<DocumentRecord>, DocumentError> {
        self.repo.find_by_type(operator_id, category).await
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError> {
        self.repo.find_by_operator(operator_id).await
    }
}

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_document(&self, id: Uuid) -> Result<DocumentRecord, DocumentError> {
        (**self).get_document(id).await
    }
    async fn find_by_type(&self, operator_id: Uuid, category: &str) -> Result<Vec<DocumentRecord>, DocumentError> {
        (**self).find_by_type(operator_id, category).await
    }
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError> {
        (**self).find_by_operator(operator_id).await
    }
}
