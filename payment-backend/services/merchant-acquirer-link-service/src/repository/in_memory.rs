//! In-memory repository for MerchantAcquirerLink.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{MerchantAcquirerLink, LinkError};
use crate::repository::traits::LinkRepository;

#[derive(Clone)]
pub struct InMemoryLinkRepository {
    pub(super) links: Arc<RwLock<HashMap<Uuid, MerchantAcquirerLink>>>,
}

impl InMemoryLinkRepository {
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryLinkRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LinkRepository for InMemoryLinkRepository {
    async fn load(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        let map = self.links.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, link: &MerchantAcquirerLink) -> Result<(), LinkError> {
        let mut map = self.links.write().await;
        map.insert(link.link_id, link.clone());
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let map = self.links.read().await;
        let mut links: Vec<MerchantAcquirerLink> = map.values()
            .filter(|l| l.operator_id == operator_id)
            .cloned()
            .collect();
        links.sort_by_key(|a| a.created_at);
        Ok(links)
    }

    async fn find_active_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let map = self.links.read().await;
        let links: Vec<MerchantAcquirerLink> = map.values()
            .filter(|l| l.operator_id == operator_id && l.status.is_routable())
            .cloned()
            .collect();
        Ok(links)
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let map = self.links.read().await;
        let links: Vec<MerchantAcquirerLink> = map.values()
            .filter(|l| l.connector_id == connector_id)
            .cloned()
            .collect();
        Ok(links)
    }

    async fn find_by_credentials_hash(&self, hash: &str) -> Result<Option<MerchantAcquirerLink>, LinkError> {
        let map = self.links.read().await;
        Ok(map.values().find(|l| l.credentials_hash == hash).cloned())
    }

    async fn find_expired_credentials(&self) -> Result<Vec<MerchantAcquirerLink>, LinkError> {
        let map = self.links.read().await;
        let now = chrono::Utc::now();
        let links: Vec<MerchantAcquirerLink> = map.values()
            .filter(|l| l.credentials_expires_at.is_some_and(|exp| exp < now))
            .cloned()
            .collect();
        Ok(links)
    }

    async fn count_active_by_connector(&self, operator_id: Uuid, connector_id: &str) -> Result<usize, LinkError> {
        let map = self.links.read().await;
        let count = map.values()
            .filter(|l| l.operator_id == operator_id && l.connector_id == connector_id && l.status.is_routable())
            .count();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MerchantAcquirerLink, LinkEnvironment};

    #[tokio::test]
    async fn test_save_and_load() {
        let repo = InMemoryLinkRepository::new();
        let link = MerchantAcquirerLink::new(
            Uuid::now_v7(), "checkout_com".into(),
            "Test".into(), LinkEnvironment::Sandbox,
            vec![1], "hash".into(),
        );
        repo.save(&link).await.unwrap();
        let loaded = repo.load(link.link_id).await.unwrap().unwrap();
        assert_eq!(loaded.link_id, link.link_id);
    }

    #[tokio::test]
    async fn test_find_by_operator() {
        let repo = InMemoryLinkRepository::new();
        let op_id = Uuid::now_v7();
        let link = MerchantAcquirerLink::new(
            op_id, "checkout_com".into(),
            "Test".into(), LinkEnvironment::Sandbox,
            vec![1], "hash".into(),
        );
        repo.save(&link).await.unwrap();
        let links = repo.find_by_operator(op_id).await.unwrap();
        assert_eq!(links.len(), 1);
    }
}
