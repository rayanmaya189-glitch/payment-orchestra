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
    async fn load(&self, id: Uuid) -> Result<Option<RoutingPolicy>, PlatformError>;
    async fn load_active_for_operator(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, PlatformError>;
    async fn save(&self, policy: &RoutingPolicy) -> Result<(), PlatformError>;
}

/// Append-only event store repository per SRS Part 5, §4.
/// Events are immutable — only append is allowed.
#[async_trait]
pub trait EventStoreRepository: Send + Sync {
    /// Append an event to the store with optimistic concurrency control.
    /// `expected_sequence` is the last known sequence for this aggregate.
    /// Returns the assigned sequence number.
    async fn append(
        &self,
        event: &shared_types::events::EventEnvelope,
        expected_sequence: i64,
    ) -> Result<i64, PlatformError>;

    /// Load all events for an aggregate, ordered by sequence.
    async fn load_events(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Vec<shared_types::events::EventEnvelope>, PlatformError>;

    /// Get the current sequence number for an aggregate (0 if no events).
    async fn current_sequence(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<i64, PlatformError>;
}
