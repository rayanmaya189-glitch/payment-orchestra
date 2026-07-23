//! In-memory repository for IAM aggregates.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{Principal, PendingChange, ApiKey, IamError};
use crate::repository::traits::IamRepository;

#[derive(Clone)]
pub struct InMemoryIamRepository {
    pub(super) principals: Arc<RwLock<HashMap<Uuid, Principal>>>,
    pub(super) email_index: Arc<RwLock<HashMap<String, Uuid>>>,
    pub(super) api_keys: Arc<RwLock<HashMap<Uuid, ApiKey>>>,
    pub(super) pending_changes: Arc<RwLock<HashMap<Uuid, PendingChange>>>,
}

impl InMemoryIamRepository {
    pub fn new() -> Self {
        Self {
            principals: Arc::new(RwLock::new(HashMap::new())),
            email_index: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            pending_changes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryIamRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl IamRepository for InMemoryIamRepository {
    async fn load_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError> {
        let map = self.principals.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save_principal(&self, principal: &Principal) -> Result<(), IamError> {
        let mut map = self.principals.write().await;
        let mut idx = self.email_index.write().await;
        if let Some(existing) = map.get(&principal.id) {
            if let Some(ref email) = existing.email {
                idx.remove(email);
            }
        }
        if let Some(ref email) = principal.email {
            idx.insert(email.clone(), principal.id);
        }
        map.insert(principal.id, principal.clone());
        Ok(())
    }

    async fn find_principal_by_email(&self, email: &str) -> Result<Option<Principal>, IamError> {
        let idx = self.email_index.read().await;
        if let Some(id) = idx.get(email) {
            self.load_principal(*id).await
        } else {
            Ok(None)
        }
    }

    async fn load_api_key(&self, id: Uuid) -> Result<Option<ApiKey>, IamError> {
        let map = self.api_keys.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save_api_key(&self, key: &ApiKey) -> Result<(), IamError> {
        let mut map = self.api_keys.write().await;
        map.insert(key.api_key_id, key.clone());
        Ok(())
    }

    async fn list_api_keys_for_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError> {
        let map = self.api_keys.read().await;
        let keys: Vec<ApiKey> = map.values()
            .filter(|k| k.principal_id == principal_id)
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn find_api_key_by_name(&self, principal_id: Uuid, name: &str) -> Result<Option<ApiKey>, IamError> {
        let map = self.api_keys.read().await;
        Ok(map.values()
            .find(|k| k.principal_id == principal_id && k.name == name)
            .cloned())
    }

    async fn load_change(&self, id: Uuid) -> Result<Option<PendingChange>, IamError> {
        let map = self.pending_changes.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save_change(&self, change: &PendingChange) -> Result<(), IamError> {
        let mut map = self.pending_changes.write().await;
        map.insert(change.change_id, change.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Principal;

    #[tokio::test]
    async fn test_save_and_load_principal() {
        let repo = InMemoryIamRepository::new();
        let p = Principal::new_human(Uuid::now_v7(), "test@test.com".into(), vec![1, 2, 3]);
        repo.save_principal(&p).await.unwrap();
        let loaded = repo.load_principal(p.id).await.unwrap().unwrap();
        assert_eq!(loaded.email.unwrap(), "test@test.com");
    }

    #[tokio::test]
    async fn test_find_by_email() {
        let repo = InMemoryIamRepository::new();
        let p = Principal::new_human(Uuid::now_v7(), "find@test.com".into(), vec![1, 2, 3]);
        repo.save_principal(&p).await.unwrap();
        let found = repo.find_principal_by_email("find@test.com").await.unwrap().unwrap();
        assert_eq!(found.id, p.id);
    }
}
