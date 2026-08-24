//! In-memory Payment Link repository — BC-07

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::traits::PaymentLinkRepository;

#[derive(Clone)]
pub struct InMemoryPaymentLinkRepository {
    pub(super) links: Arc<RwLock<HashMap<Uuid, PaymentLink>>>,
    pub(super) token_index: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl Default for InMemoryPaymentLinkRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryPaymentLinkRepository {
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(HashMap::new())),
            token_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl PaymentLinkRepository for InMemoryPaymentLinkRepository {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let map = self.links.read().await;
        Ok(map.get(&id).cloned())
    }

    async fn load_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let token_map = self.token_index.read().await;
        match token_map.get(token) {
            Some(id) => {
                let map = self.links.read().await;
                Ok(map.get(id).cloned())
            }
            None => Ok(None),
        }
    }

    async fn save(&self, link: &PaymentLink) -> Result<(), PaymentLinkError> {
        let id = link.payment_link_id;
        let token = link.token.clone();
        {
            let mut map = self.links.write().await;
            map.insert(id, link.clone());
        }
        {
            let mut token_map = self.token_index.write().await;
            token_map.insert(token, id);
        }
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let map = self.links.read().await;
        let results: Vec<PaymentLink> = map.values()
            .filter(|l| l.operator_id == operator_id)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn find_expired(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let map = self.links.read().await;
        let now = Utc::now();
        let expired: Vec<PaymentLink> = map.values()
            .filter(|l| l.is_expired(&now) || l.status == PaymentLinkStatus::Expired)
            .cloned()
            .collect();
        Ok(expired)
    }
}
