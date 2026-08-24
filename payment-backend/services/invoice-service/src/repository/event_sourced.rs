//! Event-sourced invoice repository using platform-event-store.
//!
//! Implements InvoiceRepository by appending domain events to the event store
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
use crate::events::InvoiceEvent;
use crate::repository::InvoiceRepository;

/// Event-sourced invoice repository.
pub struct EventSourcedInvoiceRepository {
    event_store: PlatformEventStore,
    // Projection indexes for efficient queries
    invoices_by_operator: Arc<tokio::sync::RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    invoices_by_order_ref: Arc<tokio::sync::RwLock<HashMap<String, Uuid>>>,
    invoices_by_payment_intent: Arc<tokio::sync::RwLock<HashMap<Uuid, Uuid>>>,
    invoices_cache: Arc<tokio::sync::RwLock<HashMap<Uuid, Invoice>>>,
}

impl EventSourcedInvoiceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            event_store: PlatformEventStore::new(db),
            invoices_by_operator: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            invoices_by_order_ref: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            invoices_by_payment_intent: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            invoices_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Update projection indexes when an invoice is loaded or saved.
    async fn update_projections(&self, invoice: &Invoice) {
        let mut by_operator = self.invoices_by_operator.write().await;
        by_operator
            .entry(invoice.operator_id)
            .or_insert_with(Vec::new)
            .push(invoice.invoice_id);

        if !invoice.order_reference.is_empty() {
            let mut by_order_ref = self.invoices_by_order_ref.write().await;
            by_order_ref.insert(invoice.order_reference.clone(), invoice.invoice_id);
        }

        for pi_id in &invoice.payment_intent_ids {
            let mut by_payment_intent = self.invoices_by_payment_intent.write().await;
            by_payment_intent.insert(*pi_id, invoice.invoice_id);
        }

        let mut cache = self.invoices_cache.write().await;
        cache.insert(invoice.invoice_id, invoice.clone());
    }

    async fn append_events(
        &self,
        invoice_id: Uuid,
        events: &[InvoiceEvent],
    ) -> Result<(), InvoiceError> {
        let latest = self
            .event_store
            .latest_sequence("Invoice", invoice_id)
            .await
            .map_err(InvoiceError::DatabaseError)?;

        let start_seq = latest.unwrap_or(0) + 1;

        let stored_events: Vec<StoredEvent> = events
            .iter()
            .enumerate()
            .map(|(i, event)| event_to_stored_event(invoice_id, event, start_seq + i as i64))
            .collect();

        self.event_store
            .append_events(stored_events)
            .await
            .map_err(InvoiceError::DatabaseError)?;

        Ok(())
    }
}

#[async_trait]
impl InvoiceRepository for EventSourcedInvoiceRepository {
    async fn load_invoice(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
        // Check cache first
        {
            let cache = self.invoices_cache.read().await;
            if let Some(invoice) = cache.get(&id) {
                return Ok(Some(invoice.clone()));
            }
        }

        let stored_events = self
            .event_store
            .read_all_events("Invoice", id)
            .await
            .map_err(InvoiceError::DatabaseError)?;

        if stored_events.is_empty() {
            return Ok(None);
        }

        let mut invoice = Invoice {
            invoice_id: id,
            operator_id: Default::default(),
            order_reference: String::new(),
            status: InvoiceStatus::Draft,
            line_items: Vec::new(),
            total_amount_minor: 0,
            paid_amount_minor: 0,
            currency: "AED".into(),
            due_date: Utc::now(),
            recipient_email: None,
            payment_intent_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            pending_events: Vec::new(),
        };

        for stored in &stored_events {
            let event: InvoiceEvent = serde_json::from_slice(&stored.payload)
                .map_err(|e| InvoiceError::DatabaseError(format!(
                    "Failed to deserialize event: {}", e
                )))?;
            invoice.apply_event(&event);
        }

        // Update projections
        self.update_projections(&invoice).await;

        Ok(Some(invoice))
    }

    async fn save_invoice(&self, invoice: &mut Invoice) -> Result<(), InvoiceError> {
        if invoice.pending_events.is_empty() {
            return Ok(());
        }
        let events = std::mem::take(&mut invoice.pending_events);
        self.append_events(invoice.invoice_id, &events).await?;
        // Update projections after saving
        self.update_projections(invoice).await;
        Ok(())
    }

    async fn find_by_order_reference(
        &self,
        _operator_id: Uuid,
        order_ref: &str,
    ) -> Result<Option<Invoice>, InvoiceError> {
        let by_order_ref = self.invoices_by_order_ref.read().await;
        let cache = self.invoices_cache.read().await;
        if let Some(invoice_id) = by_order_ref.get(order_ref) {
            return Ok(cache.get(invoice_id).cloned());
        }
        Ok(None)
    }

    async fn find_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<Invoice>, InvoiceError> {
        let by_payment_intent = self.invoices_by_payment_intent.read().await;
        let cache = self.invoices_cache.read().await;
        if let Some(invoice_id) = by_payment_intent.get(&payment_intent_id) {
            return Ok(cache.get(invoice_id).cloned());
        }
        Ok(None)
    }

    async fn find_overdue(&self, operator_id: Uuid) -> Result<Vec<Invoice>, InvoiceError> {
        let by_operator = self.invoices_by_operator.read().await;
        let cache = self.invoices_cache.read().await;
        let now = Utc::now();
        if let Some(invoice_ids) = by_operator.get(&operator_id) {
            let invoices: Vec<Invoice> = invoice_ids
                .iter()
                .filter_map(|id| cache.get(id))
                .filter(|inv| inv.due_date < now && inv.status != InvoiceStatus::Paid)
                .cloned()
                .collect();
            return Ok(invoices);
        }
        Ok(Vec::new())
    }

    async fn list_invoices(
        &self,
        operator_id: Uuid,
        status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        let by_operator = self.invoices_by_operator.read().await;
        let cache = self.invoices_cache.read().await;
        if let Some(invoice_ids) = by_operator.get(&operator_id) {
            let invoices: Vec<Invoice> = invoice_ids
                .iter()
                .filter_map(|id| cache.get(id))
                .filter(|inv| status_filter.as_ref().map_or(true, |s| inv.status == *s))
                .cloned()
                .collect();
            return Ok(invoices);
        }
        Ok(Vec::new())
    }
}

// ─── Event Conversion Helpers ────────────────────────────────────────────────

fn event_to_stored_event(
    invoice_id: Uuid,
    event: &InvoiceEvent,
    sequence: i64,
) -> StoredEvent {
    StoredEvent {
        event_id: Uuid::now_v7(),
        aggregate_type: "Invoice".into(),
        aggregate_id: invoice_id,
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
        let inv_id = Uuid::now_v7();
        let event = InvoiceEvent::InvoiceCreated(crate::events::InvoiceCreated {
            invoice_id: inv_id,
            operator_id: Uuid::now_v7(),
            order_reference: "ORD-001".into(),
            total_amount_minor: 1000,
            currency: "AED".into(),
            due_date: now,
            recipient_email: None,
            occurred_at: now,
        });

        let stored = event_to_stored_event(inv_id, &event, 1);
        let deserialized: InvoiceEvent = serde_json::from_slice(&stored.payload).unwrap();
        assert!(matches!(deserialized, InvoiceEvent::InvoiceCreated(_)));
    }
}
