//! In-memory Onboarding repository.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::OnboardingRepository;

#[derive(Clone)]
pub struct InMemoryOnboardingRepository {
    pub(super) requests: Arc<RwLock<HashMap<Uuid, OnboardingRequest>>>,
}

impl Default for InMemoryOnboardingRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryOnboardingRepository {
    pub fn new() -> Self {
        Self { requests: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl OnboardingRepository for InMemoryOnboardingRepository {
    async fn load(&self, id: Uuid) -> Result<Option<OnboardingRequest>, OnboardingError> {
        let map = self.requests.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn save(&self, request: &OnboardingRequest) -> Result<(), OnboardingError> {
        let mut map = self.requests.write().await;
        map.insert(request.link_id, request.clone());
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let map = self.requests.read().await;
        Ok(map.values().filter(|r| r.operator_id == operator_id).cloned().collect())
    }

    async fn find_active(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError> {
        let map = self.requests.read().await;
        Ok(map.values().filter(|r| r.operator_id == operator_id && r.status == OnboardingStatus::Active).cloned().collect())
    }
}
