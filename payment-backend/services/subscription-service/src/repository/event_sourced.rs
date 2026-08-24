//! Event-sourced subscription repository using platform-event-store.
//!
//! Implements SubscriptionRepository by appending domain events to the event store
//! and rebuilding aggregate state from event streams.

#![allow(clippy::too_many_lines)]

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use platform_event_store::{EventStore as PlatformEventStore, StoredEvent};

use crate::domain::*;
use crate::events::SubscriptionEvent;
use crate::repository::SubscriptionRepository;

/// Event-sourced subscription repository.
pub struct EventSourcedSubscriptionRepository {
    event_store: PlatformEventStore,
    // Projection indexes for efficient queries
    subs_by_operator: Arc<tokio::sync::RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    subs_by_customer: Arc<tokio::sync::RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    active_subs: Arc<tokio::sync::RwLock<Vec<Uuid>>>,
    subs_cache: Arc<tokio::sync::RwLock<HashMap<Uuid, Subscription>>>,
}

impl EventSourcedSubscriptionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            event_store: PlatformEventStore::new(db),
            subs_by_operator: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            subs_by_customer: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            active_subs: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            subs_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Update projection indexes when a subscription is loaded or saved.
    async fn update_projections(&self, sub: &Subscription) {
        let mut by_operator = self.subs_by_operator.write().await;
        by_operator
            .entry(sub.operator_id)
            .or_insert_with(Vec::new)
            .push(sub.subscription_id);

        let mut by_customer = self.subs_by_customer.write().await;
        by_customer
            .entry(sub.customer_id)
            .or_insert_with(Vec::new)
            .push(sub.subscription_id);

        if sub.status == SubscriptionStatus::Active {
            let mut active = self.active_subs.write().await;
            if !active.contains(&sub.subscription_id) {
                active.push(sub.subscription_id);
            }
        }

        let mut cache = self.subs_cache.write().await;
        cache.insert(sub.subscription_id, sub.clone());
    }

    /// Append subscription events to the event store.
    async fn append_events(
        &self,
        subscription_id: Uuid,
        events: &[SubscriptionEvent],
    ) -> Result<(), SubscriptionError> {
        let latest = self
            .event_store
            .latest_sequence("Subscription", subscription_id)
            .await
            .map_err(SubscriptionError::DatabaseError)?;

        let start_seq = latest.unwrap_or(0) + 1;

        let stored_events: Vec<StoredEvent> = events
            .iter()
            .enumerate()
            .map(|(i, event)| event_to_stored_event(subscription_id, event, start_seq + i as i64))
            .collect();

        self.event_store
            .append_events(stored_events)
            .await
            .map_err(SubscriptionError::DatabaseError)?;

        Ok(())
    }
}

#[async_trait]
impl SubscriptionRepository for EventSourcedSubscriptionRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Subscription>, SubscriptionError> {
        // Check cache first
        {
            let cache = self.subs_cache.read().await;
            if let Some(sub) = cache.get(&id) {
                return Ok(Some(sub.clone()));
            }
        }

        let stored_events = self
            .event_store
            .read_all_events("Subscription", id)
            .await
            .map_err(SubscriptionError::DatabaseError)?;

        if stored_events.is_empty() {
            return Ok(None);
        }

        // Create minimal subscription to start applying events
        let mut sub = Subscription {
            subscription_id: id,
            operator_id: Default::default(),
            customer_id: Default::default(),
            plan_id: String::new(),
            plan_amount_minor_units: 0,
            currency: "AED".into(),
            status: SubscriptionStatus::Active,
            current_period_start: Utc::now(),
            current_period_end: Utc::now(),
            billing_interval_days: 30,
            payment_method_token_id: None,
            dunning_retry_count: 0,
            max_dunning_retries: 3,
            billing_cycles: Vec::new(),
            dunning_retries: Vec::new(),
            created_at: Utc::now(),
            cancelled_at: None,
            paused_at: None,
            resumed_at: None,
            pending_events: Vec::new(),
        };

        for stored in &stored_events {
            let event: SubscriptionEvent = serde_json::from_slice(&stored.payload)
                .map_err(|e| SubscriptionError::DatabaseError(format!(
                    "Failed to deserialize event: {}", e
                )))?;
            sub.apply_event(&event);
        }

        // Update projections
        self.update_projections(&sub).await;

        Ok(Some(sub))
    }

    async fn save(&self, subscription: &mut Subscription) -> Result<(), SubscriptionError> {
        if subscription.pending_events.is_empty() {
            return Ok(());
        }
        let events = std::mem::take(&mut subscription.pending_events);
        self.append_events(subscription.subscription_id, &events).await?;
        // Update projections after saving
        self.update_projections(subscription).await;
        Ok(())
    }

    async fn find_active_for_renewal(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        let active = self.active_subs.read().await;
        let cache = self.subs_cache.read().await;
        let now = Utc::now();
        let subscriptions: Vec<Subscription> = active
            .iter()
            .filter_map(|id| cache.get(id))
            .filter(|sub| sub.current_period_end <= now)
            .cloned()
            .collect();
        Ok(subscriptions)
    }

    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let by_customer = self.subs_by_customer.read().await;
        let cache = self.subs_cache.read().await;
        if let Some(sub_ids) = by_customer.get(&customer_id) {
            let subscriptions: Vec<Subscription> = sub_ids
                .iter()
                .filter_map(|id| cache.get(id).cloned())
                .collect();
            return Ok(subscriptions);
        }
        Ok(Vec::new())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Subscription>, SubscriptionError> {
        let by_operator = self.subs_by_operator.read().await;
        let cache = self.subs_cache.read().await;
        if let Some(sub_ids) = by_operator.get(&operator_id) {
            let subscriptions: Vec<Subscription> = sub_ids
                .iter()
                .filter_map(|id| cache.get(id).cloned())
                .collect();
            return Ok(subscriptions);
        }
        Ok(Vec::new())
    }
}

// ─── Event Conversion Helpers ────────────────────────────────────────────────

fn event_to_stored_event(
    subscription_id: Uuid,
    event: &SubscriptionEvent,
    sequence: i64,
) -> StoredEvent {
    StoredEvent {
        event_id: Uuid::now_v7(),
        aggregate_type: "Subscription".into(),
        aggregate_id: subscription_id,
        event_type: event.event_type().to_string(),
        event_sequence: sequence,
        event_version: 1,
        occurred_at: event.occurred_at(),
        actor_type: "system".into(),
        actor_id: None,
        causation_id: None,
        correlation_id: Uuid::now_v7(),
        payload: serde_json::to_vec(event).unwrap_or_default(),
        encrypted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_roundtrip() {
        let now = Utc::now();
        let sub_id = Uuid::now_v7();
        let event = SubscriptionEvent::Created(crate::events::SubscriptionCreated {
            subscription_id: sub_id,
            operator_id: Uuid::now_v7(),
            customer_id: Uuid::now_v7(),
            plan_id: "plan_test".into(),
            amount_minor_units: 1000,
            currency: "AED".into(),
            current_period_start: now,
            current_period_end: now + chrono::Duration::days(30),
            payment_method_token_id: None,
            occurred_at: now,
        });

        let stored = event_to_stored_event(sub_id, &event, 1);
        let deserialized: SubscriptionEvent = serde_json::from_slice(&stored.payload).unwrap();
        assert!(matches!(deserialized, SubscriptionEvent::Created(_)));
    }
}
