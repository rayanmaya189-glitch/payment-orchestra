use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::{PaymentIntent, RoutingAttempt, RoutingPolicy};
use platform_error::PlatformError;

#[async_trait]
pub trait PaymentIntentRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentIntent>, PlatformError>;
    async fn save(&self, intent: &PaymentIntent) -> Result<(), PlatformError>;
    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<PaymentIntent>, PlatformError>;
    async fn save_attempt(&self, attempt: &RoutingAttempt) -> Result<(), PlatformError>;
    async fn load_attempts(&self, payment_intent_id: Uuid) -> Result<Vec<RoutingAttempt>, PlatformError>;
}

#[async_trait]
pub trait RoutingPolicyRepository: Send + Sync {
    async fn load_active_for_operator(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, PlatformError>;
    async fn save(&self, policy: &RoutingPolicy) -> Result<(), PlatformError>;
}
