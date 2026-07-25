//! Read-model queries for merchant-acquirer-link service.

use uuid::Uuid;

use crate::domain::{MerchantAcquirerLink, LinkError};
use crate::repository::LinkRepository;

#[async_trait::async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_link(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError>;
    async fn list_links(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<MerchantAcquirerLink>, LinkError>;
}

pub struct LinkQueries<R: LinkRepository> {
    repository: R,
}

impl<R: LinkRepository> LinkQueries<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl<R: LinkRepository + Send + Sync> QueryHandler for LinkQueries<R> {
    async fn get_link(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        self.repository.load(id).await
    }

    async fn list_links(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let all = self.repository.find_by_operator(operator_id).await?;
        match status_filter {
            Some(status) => Ok(all.into_iter().filter(|l| l.status.as_str() == status).collect()),
            None => Ok(all),
        }
    }
}

// ─── Blanket impl: Box<dyn QueryHandler> delegates to inner ──────────────────

#[async_trait::async_trait]
impl<T: QueryHandler + ?Sized> QueryHandler for Box<T> {
    async fn get_link(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        (**self).get_link(id).await
    }

    async fn list_links(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        (**self).list_links(operator_id, status_filter).await
    }
}
