//! Document Management repository trait — BC-13

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<DocumentRecord>, DocumentError>;
    async fn save(&self, record: &DocumentRecord) -> Result<(), DocumentError>;
    async fn delete(&self, id: Uuid) -> Result<(), DocumentError>;
    async fn find_by_type(&self, operator_id: Uuid, category: &str) -> Result<Vec<DocumentRecord>, DocumentError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError>;
}
