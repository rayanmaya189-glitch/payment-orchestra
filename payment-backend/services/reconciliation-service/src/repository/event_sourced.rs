//! Event-sourced reconciliation repository using platform-event-store.
//!
//! Implements SettlementBatchRepository by appending domain events to the event store
//! and rebuilding aggregate state from event streams. Other repository traits
//! (LedgerEntry, SettlementExpectation, FeeVariance) use in-memory stores
//! since they are not event-sourced.

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
use crate::events::ReconciliationEvent;
use crate::repository::{
    FeeVarianceRepository, LedgerEntryRepository, SettlementBatchRepository,
    SettlementExpectationRepository,
};

/// Event-sourced reconciliation repository.
///
/// SettlementBatchRepository is implemented via event sourcing with projections.
/// Other traits use in-memory stores.
pub struct EventSourcedReconciliationRepository {
    event_store: PlatformEventStore,
    // In-memory stores for non-event-sourced aggregates
    ledger: Arc<RwLock<Vec<LedgerEntry>>>,
    expectations: Arc<RwLock<HashMap<Uuid, SettlementExpectation>>>,
    fee_variances: Arc<RwLock<HashMap<Uuid, FeeVariance>>>,
    // Projection indexes for efficient queries
    batches_by_checksum: Arc<RwLock<HashMap<String, Uuid>>>,
    all_batch_ids: Arc<RwLock<Vec<Uuid>>>,
    batches_cache: Arc<RwLock<HashMap<Uuid, SettlementBatch>>>,
}

impl EventSourcedReconciliationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            event_store: PlatformEventStore::new(db),
            ledger: Arc::new(RwLock::new(Vec::new())),
            expectations: Arc::new(RwLock::new(HashMap::new())),
            fee_variances: Arc::new(RwLock::new(HashMap::new())),
            batches_by_checksum: Arc::new(RwLock::new(HashMap::new())),
            all_batch_ids: Arc::new(RwLock::new(Vec::new())),
            batches_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update projection indexes when a batch is loaded or saved.
    async fn update_projections(&self, batch: &SettlementBatch) {
        if !batch.file_checksum.is_empty() {
            let mut by_checksum = self.batches_by_checksum.write().await;
            by_checksum.insert(batch.file_checksum.clone(), batch.settlement_batch_id);
        }

        let mut all_ids = self.all_batch_ids.write().await;
        if !all_ids.contains(&batch.settlement_batch_id) {
            all_ids.push(batch.settlement_batch_id);
        }

        let mut cache = self.batches_cache.write().await;
        cache.insert(batch.settlement_batch_id, batch.clone());
    }

    async fn append_events(
        &self,
        batch_id: Uuid,
        events: &[ReconciliationEvent],
    ) -> Result<(), ReconciliationError> {
        let latest = self
            .event_store
            .latest_sequence("SettlementBatch", batch_id)
            .await
            .map_err(ReconciliationError::DatabaseError)?;

        let start_seq = latest.unwrap_or(0) + 1;

        let stored_events: Vec<StoredEvent> = events
            .iter()
            .enumerate()
            .map(|(i, event)| event_to_stored_event(batch_id, event, start_seq + i as i64))
            .collect();

        self.event_store
            .append_events(stored_events)
            .await
            .map_err(ReconciliationError::DatabaseError)?;

        Ok(())
    }
}

#[async_trait]
impl SettlementBatchRepository for EventSourcedReconciliationRepository {
    async fn load_settlement_batch(&self, id: Uuid) -> Result<Option<SettlementBatch>, ReconciliationError> {
        // Check cache first
        {
            let cache = self.batches_cache.read().await;
            if let Some(batch) = cache.get(&id) {
                return Ok(Some(batch.clone()));
            }
        }

        let stored_events = self
            .event_store
            .read_all_events("SettlementBatch", id)
            .await
            .map_err(ReconciliationError::DatabaseError)?;

        if stored_events.is_empty() {
            return Ok(None);
        }

        let mut batch: Option<SettlementBatch> = None;
        for stored in &stored_events {
            let event: ReconciliationEvent = serde_json::from_slice(&stored.payload)
                .map_err(|e| ReconciliationError::DatabaseError(format!(
                    "Failed to deserialize event: {}", e
                )))?;

            if let Some(ref mut b) = batch {
                b.apply_event(&event);
            } else {
                // First event creates the aggregate
                let mut b = SettlementBatch {
                    settlement_batch_id: id,
                    operator_id: Default::default(),
                    acquirer_link_id: Default::default(),
                    file_checksum: String::new(),
                    file_format: String::new(),
                    status: BatchStatus::Ingesting,
                    total_records: 0,
                    matched_count: 0,
                    unmatched_count: 0,
                    total_amount_minor: 0,
                    records: Vec::new(),
                    ingested_at: Utc::now(),
                    processed_at: None,
                    pending_events: Vec::new(),
                };
                b.apply_event(&event);
                batch = Some(b);
            }
        }

        // Update projections
        if let Some(ref b) = batch {
            self.update_projections(b).await;
        }

        Ok(batch)
    }

    async fn save_settlement_batch(&self, batch: &mut SettlementBatch) -> Result<(), ReconciliationError> {
        if batch.pending_events.is_empty() {
            return Ok(());
        }
        let events = std::mem::take(&mut batch.pending_events);
        self.append_events(batch.settlement_batch_id, &events).await?;
        // Update projections after saving
        self.update_projections(batch).await;
        Ok(())
    }

    async fn find_batch_by_checksum(&self, checksum: &str) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let by_checksum = self.batches_by_checksum.read().await;
        let cache = self.batches_cache.read().await;
        if let Some(batch_id) = by_checksum.get(checksum) {
            return Ok(cache.get(batch_id).cloned());
        }
        Ok(None)
    }

    async fn list_all_batches(&self) -> Result<Vec<SettlementBatch>, ReconciliationError> {
        let all_ids = self.all_batch_ids.read().await;
        let cache = self.batches_cache.read().await;
        let batches: Vec<SettlementBatch> = all_ids
            .iter()
            .filter_map(|id| cache.get(id).cloned())
            .collect();
        Ok(batches)
    }
}

#[async_trait]
impl LedgerEntryRepository for EventSourcedReconciliationRepository {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError> {
        self.ledger.write().await.push(entry.clone());
        Ok(())
    }

    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError> {
        let ledger = self.ledger.read().await;
        let total_debit: i64 = ledger.iter()
            .filter(|e| e.transaction_id == transaction_id && e.entry_type == EntryType::Debit)
            .map(|e| e.amount_minor)
            .sum();
        let total_credit: i64 = ledger.iter()
            .filter(|e| e.transaction_id == transaction_id && e.entry_type == EntryType::Credit)
            .map(|e| e.amount_minor)
            .sum();
        Ok(total_debit == total_credit)
    }

    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError> {
        let ledger = self.ledger.read().await;
        let mut txn_ids: Vec<Uuid> = ledger.iter().map(|e| e.transaction_id).collect();
        txn_ids.sort();
        txn_ids.dedup();

        let mut imbalanced = Vec::new();
        for txn_id in txn_ids {
            let total_debit: i64 = ledger.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == EntryType::Debit)
                .map(|e| e.amount_minor)
                .sum();
            let total_credit: i64 = ledger.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == EntryType::Credit)
                .map(|e| e.amount_minor)
                .sum();
            if total_debit != total_credit {
                imbalanced.push(txn_id);
            }
        }
        Ok(imbalanced)
    }
}

#[async_trait]
impl SettlementExpectationRepository for EventSourcedReconciliationRepository {
    async fn save_settlement_expectation(&self, expectation: &SettlementExpectation) -> Result<(), ReconciliationError> {
        self.expectations.write().await.insert(expectation.expectation_id, expectation.clone());
        Ok(())
    }

    async fn load_settlement_expectation(&self, id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let store = self.expectations.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_expectation_by_payment(&self, payment_intent_id: Uuid) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let store = self.expectations.read().await;
        Ok(store.values().find(|e| e.payment_intent_id == payment_intent_id).cloned())
    }

    async fn find_overdue_expectations(&self) -> Result<Vec<SettlementExpectation>, ReconciliationError> {
        let store = self.expectations.read().await;
        let now = Utc::now();
        Ok(store.values()
            .filter(|e| e.status == ExpectationStatus::Pending && e.expected_settlement_date < now)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl FeeVarianceRepository for EventSourcedReconciliationRepository {
    async fn save_fee_variance(&self, variance: &FeeVariance) -> Result<(), ReconciliationError> {
        self.fee_variances.write().await.insert(variance.variance_id, variance.clone());
        Ok(())
    }

    async fn load_fee_variance(&self, id: Uuid) -> Result<Option<FeeVariance>, ReconciliationError> {
        let store = self.fee_variances.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_fee_variances_for_payment(&self, payment_intent_id: Uuid) -> Result<Vec<FeeVariance>, ReconciliationError> {
        let store = self.fee_variances.read().await;
        Ok(store.values().filter(|v| v.payment_intent_id == payment_intent_id).cloned().collect())
    }
}

// ─── Event Conversion Helpers ────────────────────────────────────────────────

fn event_to_stored_event(
    batch_id: Uuid,
    event: &ReconciliationEvent,
    sequence: i64,
) -> StoredEvent {
    StoredEvent {
        event_id: Uuid::now_v7(),
        aggregate_type: "SettlementBatch".into(),
        aggregate_id: batch_id,
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
    fn test_event_type_method() {
        let now = Utc::now();
        let event = ReconciliationEvent::SettlementBatchIngested(crate::events::SettlementBatchIngested {
            settlement_batch_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            acquirer_link_id: Uuid::now_v7(),
            total_records: 10,
            total_amount_minor: 10000,
            file_format: "csv".into(),
            occurred_at: now,
        });

        assert_eq!(event.event_type(), "settlement.batch_ingested");
    }

    #[test]
    fn test_event_to_stored_event_roundtrip() {
        let now = Utc::now();
        let batch_id = Uuid::now_v7();
        let event = ReconciliationEvent::SettlementBatchIngested(crate::events::SettlementBatchIngested {
            settlement_batch_id: batch_id,
            operator_id: Uuid::now_v7(),
            acquirer_link_id: Uuid::now_v7(),
            total_records: 5,
            total_amount_minor: 5000,
            file_format: "csv".into(),
            occurred_at: now,
        });

        let stored = event_to_stored_event(batch_id, &event, 1);
        assert_eq!(stored.aggregate_type, "SettlementBatch");
        assert_eq!(stored.aggregate_id, batch_id);
        assert_eq!(stored.event_sequence, 1);
        assert_eq!(stored.occurred_at, now);

        let deserialized: ReconciliationEvent = serde_json::from_slice(&stored.payload).unwrap();
        assert!(matches!(deserialized, ReconciliationEvent::SettlementBatchIngested(_)));
    }
}
