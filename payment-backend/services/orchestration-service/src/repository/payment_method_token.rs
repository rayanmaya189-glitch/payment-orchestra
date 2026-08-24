//! PaymentMethodTokenRepository implementation for InMemoryOrchestrationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryOrchestrationRepository;
use super::traits::PaymentMethodTokenRepository;

#[async_trait]
impl PaymentMethodTokenRepository for InMemoryOrchestrationRepository {
    async fn load_payment_method_token(&self, id: Uuid) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        let store = self.tokens.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save_payment_method_token(&self, token: &PaymentMethodToken) -> Result<(), OrchestrationError> {
        let mut store = self.tokens.write().await;
        store.insert(token.token_id, token.clone());
        Ok(())
    }

    async fn find_active_tokens_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        let store = self.tokens.read().await;
        Ok(store.values()
            .filter(|t| t.operator_id == operator_id && t.token_status == TokenStatus::Active)
            .cloned()
            .collect())
    }
}
