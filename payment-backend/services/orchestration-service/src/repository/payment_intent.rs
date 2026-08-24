//! PaymentIntentRepository implementation for InMemoryOrchestrationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryOrchestrationRepository;
use super::traits::PaymentIntentRepository;

#[async_trait]
impl PaymentIntentRepository for InMemoryOrchestrationRepository {
    async fn load_payment_intent(&self, id: Uuid) -> Result<Option<PaymentIntent>, OrchestrationError> {
        let store = self.payment_intents.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save_payment_intent(&self, intent: &mut PaymentIntent) -> Result<(), OrchestrationError> {
        let mut store = self.payment_intents.write().await;
        store.insert(intent.payment_intent_id, intent.clone());
        Ok(())
    }

    async fn list_payment_intents_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        let store = self.payment_intents.read().await;
        Ok(store.values().filter(|pi| pi.operator_id == operator_id).cloned().collect())
    }
}
