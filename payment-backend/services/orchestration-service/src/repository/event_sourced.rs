//! Event-sourced orchestration repository using platform-event-store.
//!
//! Implements PaymentIntentRepository by appending domain events to the event store
//! and rebuilding aggregate state from event streams. Other repository traits
//! (RoutingPolicy, PaymentMethodToken, IdempotencyCache, AcquirerLinkProvider)
//! use in-memory stores since they are not event-sourced.
//!
//! DB-001: Events are appended with monotonically increasing sequence numbers.
//! DB-002: Aggregates are rebuilt from events on every load (snapshotting TBD).

#![allow(clippy::too_many_lines)]

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use platform_event_store::{EventStore as PlatformEventStore, StoredEvent};

use crate::domain::*;
use crate::repository::{
    AcquirerLinkProvider, IdempotencyCache, PaymentIntentRepository,
    PaymentMethodTokenRepository, RoutingPolicyRepository,
};

/// Event-sourced orchestration repository.
///
/// PaymentIntentRepository is implemented via event sourcing.
/// Other traits use in-memory stores.
pub struct EventSourcedOrchestrationRepository {
    event_store: PlatformEventStore,
    // In-memory stores for non-event-sourced aggregates
    routing_policies: Arc<RwLock<HashMap<Uuid, RoutingPolicy>>>,
    active_policies: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    tokens: Arc<RwLock<HashMap<Uuid, PaymentMethodToken>>>,
    idempotency_cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    active_links: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
}

impl EventSourcedOrchestrationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            event_store: PlatformEventStore::new(db),
            routing_policies: Arc::new(RwLock::new(HashMap::new())),
            active_policies: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            idempotency_cache: Arc::new(RwLock::new(HashMap::new())),
            active_links: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ─── PaymentIntentRepository (Event-Sourced) ─────────────────────────────────

#[async_trait]
impl PaymentIntentRepository for EventSourcedOrchestrationRepository {
    async fn load_payment_intent(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentIntent>, OrchestrationError> {
        let stored_events = self
            .event_store
            .read_all_events("PaymentIntent", id)
            .await
            .map_err(OrchestrationError::DatabaseError)?;

        if stored_events.is_empty() {
            return Ok(None);
        }

        let mut intent: Option<PaymentIntent> = None;
        for stored in &stored_events {
            let event: PaymentEvent = serde_json::from_slice(&stored.payload)
                .map_err(|e| {
                    OrchestrationError::DatabaseError(format!(
                        "Failed to deserialize event: {}",
                        e
                    ))
                })?;

            if let Some(ref mut pi) = intent {
                pi.apply_event(&event);
            } else {
                // First event creates the aggregate
                let mut pi = create_empty_payment_intent(id);
                pi.apply_event(&event);
                intent = Some(pi);
            }
        }

        Ok(intent)
    }

    async fn save_payment_intent(
        &self,
        intent: &mut PaymentIntent,
    ) -> Result<(), OrchestrationError> {
        // Drain pending events and append them to the event store.
        // pending_events is populated by PaymentIntent::apply_event().
        if intent.pending_events.is_empty() {
            return Ok(());
        }
        let events = std::mem::take(&mut intent.pending_events);
        self.append_payment_events(intent.payment_intent_id, &events).await
    }

    async fn list_payment_intents_for_operator(
        &self,
        _operator_id: Uuid,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        // TODO: Requires a projection or snapshot store for efficient listing.
        // Phase 2: implement a `payment_intent_by_operator` projection
        // that subscribes to PaymentIntent events and maintains a lookup index.
        Ok(Vec::new())
    }
}

impl EventSourcedOrchestrationRepository {
    /// Append payment events to the event store.
    /// Call this from the command handler after producing events.
    pub async fn append_payment_events(
        &self,
        payment_intent_id: Uuid,
        events: &[PaymentEvent],
    ) -> Result<(), OrchestrationError> {
        let latest = self
            .event_store
            .latest_sequence("PaymentIntent", payment_intent_id)
            .await
            .map_err(OrchestrationError::DatabaseError)?;

        let start_seq = latest.unwrap_or(0) + 1;

        let stored_events: Vec<StoredEvent> = events
            .iter()
            .enumerate()
            .map(|(i, event)| {
                payment_event_to_stored_event(
                    payment_intent_id,
                    event,
                    start_seq + i as i64,
                )
            })
            .collect();

        self.event_store
            .append_events(stored_events)
            .await
            .map_err(OrchestrationError::DatabaseError)?;

        Ok(())
    }
}

// ─── RoutingPolicyRepository (In-Memory — not event-sourced) ─────────────────

#[async_trait]
impl RoutingPolicyRepository for EventSourcedOrchestrationRepository {
    async fn load_active_routing_policy(
        &self,
        operator_id: Uuid,
    ) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let active = self.active_policies.read().await;
        if let Some(policy_id) = active.get(&operator_id) {
            let policies = self.routing_policies.read().await;
            return Ok(policies.get(policy_id).cloned());
        }
        Ok(None)
    }

    async fn save_routing_policy(
        &self,
        policy: &RoutingPolicy,
    ) -> Result<(), OrchestrationError> {
        let mut policies = self.routing_policies.write().await;
        policies.insert(policy.routing_policy_id, policy.clone());

        if policy.status == PolicyStatus::Active {
            let mut active = self.active_policies.write().await;
            active.insert(policy.operator_id, policy.routing_policy_id);
        }
        Ok(())
    }

    async fn load_routing_policy(
        &self,
        id: Uuid,
    ) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let policies = self.routing_policies.read().await;
        Ok(policies.get(&id).cloned())
    }
}

// ─── PaymentMethodTokenRepository (In-Memory — not event-sourced) ────────────

#[async_trait]
impl PaymentMethodTokenRepository for EventSourcedOrchestrationRepository {
    async fn load_payment_method_token(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        let tokens = self.tokens.read().await;
        Ok(tokens.get(&id).cloned())
    }

    async fn save_payment_method_token(
        &self,
        token: &PaymentMethodToken,
    ) -> Result<(), OrchestrationError> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(token.token_id, token.clone());
        Ok(())
    }

    async fn find_active_tokens_for_operator(
        &self,
        _operator_id: Uuid,
    ) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        let tokens = self.tokens.read().await;
        Ok(tokens
            .values()
            .filter(|t| t.token_status == TokenStatus::Active)
            .cloned()
            .collect())
    }
}

// ─── IdempotencyCache (In-Memory) ────────────────────────────────────────────

#[async_trait]
impl IdempotencyCache for EventSourcedOrchestrationRepository {
    async fn check_idempotency(
        &self,
        key: &str,
    ) -> Result<IdempotencyResult, OrchestrationError> {
        let cache = self.idempotency_cache.read().await;
        match cache.get(key) {
            Some(result) => Ok(IdempotencyResult::Duplicate(result.clone())),
            None => Ok(IdempotencyResult::New),
        }
    }

    async fn store_idempotency(
        &self,
        key: &str,
        result: &serde_json::Value,
    ) -> Result<(), OrchestrationError> {
        let mut cache = self.idempotency_cache.write().await;
        cache.insert(key.to_string(), result.clone());
        Ok(())
    }
}

// ─── AcquirerLinkProvider (In-Memory) ────────────────────────────────────────

#[async_trait]
impl AcquirerLinkProvider for EventSourcedOrchestrationRepository {
    async fn list_active_acquirer_links(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<Uuid>, OrchestrationError> {
        let links = self.active_links.read().await;
        Ok(links.get(&operator_id).cloned().unwrap_or_default())
    }
}

// ─── Event Conversion Helpers ────────────────────────────────────────────────

/// Convert a PaymentEvent to a StoredEvent for the event store.
fn payment_event_to_stored_event(
    payment_intent_id: Uuid,
    event: &PaymentEvent,
    sequence: i64,
) -> StoredEvent {
    let occurred_at = event.occurred_at();
    let event_type = event.event_type().to_string();
    let payload = serde_json::to_vec(event).unwrap_or_default();

    StoredEvent {
        event_id: Uuid::now_v7(),
        aggregate_type: "PaymentIntent".into(),
        aggregate_id: payment_intent_id,
        event_type,
        event_sequence: sequence,
        event_version: 1,
        occurred_at,
        actor_type: "system".into(),
        actor_id: None,
        causation_id: None,
        correlation_id: Uuid::now_v7(),
        payload,
        encrypted: false,
    }
}

/// Create an empty PaymentIntent with just the ID set.
/// Used as a starting point before applying the first event.
fn create_empty_payment_intent(id: Uuid) -> PaymentIntent {
    let now = Utc::now();
    PaymentIntent {
        payment_intent_id: id,
        operator_id: Uuid::default(),
        status: PaymentStatus::Created,
        requested_amount: Money::zero("AED"),
        authorized_amount: Money::zero("AED"),
        captured_amount: Money::zero("AED"),
        refunded_amount: Money::zero("AED"),
        currency: "AED".into(),
        idempotency_key: String::new(),
        payment_method_token_id: None,
        routing_policy_id: None,
        deployment_epoch: 0,
        purpose: PaymentPurpose::Payment,
        metadata: None,
        source_type: None,
        source_id: None,
        risk_score: None,
        risk_level: None,
        expected_settlement_date: None,
        settlement_cycle: None,
        gateway_profile_id: None,
        gateway_profile_version: None,
        gateway_rotation_strategy: None,
        gateway_selection_reason: None,
        routing_attempts: Vec::new(),
        version: 0,
        pending_events: Vec::new(),
        created_at: now,
        updated_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_occurred_at_method() {
        let now = Utc::now();
        let event = PaymentEvent::PaymentIntentCreated(PaymentIntentCreated {
            payment_intent_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 1000,
            currency: "AED".into(),
            idempotency_key: "test".into(),
            is_card_verification: false,
            source_type: None,
            source_id: None,
            metadata: None,
            occurred_at: now,
        });

        assert_eq!(event.occurred_at(), now);
    }

    #[test]
    fn test_event_to_stored_event_roundtrip() {
        let now = Utc::now();
        let pi_id = Uuid::now_v7();
        let event = PaymentEvent::PaymentIntentCreated(PaymentIntentCreated {
            payment_intent_id: pi_id,
            operator_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "USD".into(),
            idempotency_key: "key-1".into(),
            is_card_verification: false,
            source_type: None,
            source_id: None,
            metadata: None,
            occurred_at: now,
        });

        let stored = payment_event_to_stored_event(pi_id, &event, 1);
        assert_eq!(stored.aggregate_type, "PaymentIntent");
        assert_eq!(stored.aggregate_id, pi_id);
        assert_eq!(stored.event_sequence, 1);
        assert_eq!(stored.occurred_at, now);

        // Verify roundtrip
        let deserialized: PaymentEvent =
            serde_json::from_slice(&stored.payload).unwrap();
        assert!(matches!(
            deserialized,
            PaymentEvent::PaymentIntentCreated(_)
        ));
    }

    #[test]
    fn test_create_empty_payment_intent() {
        let id = Uuid::now_v7();
        let pi = create_empty_payment_intent(id);
        assert_eq!(pi.payment_intent_id, id);
        assert_eq!(pi.status, PaymentStatus::Created);
    }
}
