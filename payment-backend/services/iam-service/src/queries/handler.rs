//! Read-model queries for iam-service.

use uuid::Uuid;

use crate::domain::{Principal, ApiKey, IamError};
use crate::repository::IamRepository;

// Blanket impl: Box<dyn QueryHandler> implements QueryHandler
#[async_trait::async_trait]
impl QueryHandler for Box<dyn QueryHandler> {
    async fn get_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError> {
        self.as_ref().get_principal(id).await
    }
    async fn list_api_keys(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError> {
        self.as_ref().list_api_keys(principal_id).await
    }
}

#[async_trait::async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError>;
    async fn list_api_keys(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError>;
}

pub struct IamQueries<R: IamRepository> {
    repository: R,
}

impl<R: IamRepository> IamQueries<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl<R: IamRepository + Send + Sync> QueryHandler for IamQueries<R> {
    async fn get_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError> {
        self.repository.load_principal(id).await
    }

    async fn list_api_keys(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError> {
        self.repository.list_api_keys_for_principal(principal_id).await
    }
}
