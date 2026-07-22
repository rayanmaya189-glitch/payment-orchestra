//! Command handlers for reconciliation-service.
//! Settlement ingestion, matching, exception handling, fee variance tracking.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::*;
use crate::events::*;
use crate::repository::*;

// ─── Command Structs ─────────────────────────────────────────────────────────

/// Ingest a settlement batch from an acquirer
#[derive(Debug, Clone)]
pub struct IngestSettlementBatch {
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub raw_file: Vec<u8>,
    pub file_format: SettlementFormat,
}

/// Process matching for all unmatched records in a batch
#[derive(Debug, Clone)]
pub struct ProcessBatchMatching {
    pub settlement_batch_id: Uuid,
    pub payment_intents: Vec<PaymentIntentRef>,
}

/// Manually resolve a reconciliation exception
#[derive(Debug, Clone)]
pub struct ResolveException {
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Uuid,
    pub resolution: String,
}

/// Track fee variance after settlement matching
#[derive(Debug, Clone)]
pub struct TrackFeeVariance {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee_minor: i64,
    pub actual_fee_minor: i64,
    pub tolerance_threshold_percent: f64,
}

/// Resolve a fee variance
#[derive(Debug, Clone)]
pub struct ResolveFeeVarianceCmd {
    pub variance_id: Uuid,
    pub resolution: String,
}

/// Create a settlement expectation (for T+N tracking)
#[derive(Debug, Clone)]
pub struct CreateSettlementExpectation {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub expected_settlement_date: DateTime<Utc>,
    pub settlement_cycle: String,
}

// ─── Command Results ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestBatchResult {
    pub settlement_batch_id: Uuid,
    pub total_records: i32,
    pub total_amount_minor: i64,
    pub events: Vec<ReconciliationEvent>,
}

#[derive(Debug, Clone)]
pub struct MatchingResult {
    pub settlement_batch_id: Uuid,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub results: Vec<MatchResult>,
    pub events: Vec<ReconciliationEvent>,
}

#[derive(Debug, Clone)]
pub struct FeeVarianceResult {
    pub variance_id: Uuid,
    pub is_within_tolerance: bool,
    pub event: ReconciliationEvent,
}

// ─── Command Handler Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn ingest_settlement_batch(&self, cmd: IngestSettlementBatch) -> Result<IngestBatchResult, ReconciliationError>;
    async fn process_batch_matching(&self, cmd: ProcessBatchMatching) -> Result<MatchingResult, ReconciliationError>;
    async fn resolve_exception(&self, cmd: ResolveException) -> Result<ReconciliationEvent, ReconciliationError>;
    async fn track_fee_variance(&self, cmd: TrackFeeVariance) -> Result<FeeVarianceResult, ReconciliationError>;
    async fn resolve_fee_variance(&self, cmd: ResolveFeeVarianceCmd) -> Result<ReconciliationEvent, ReconciliationError>;
    async fn create_settlement_expectation(&self, cmd: CreateSettlementExpectation) -> Result<ReconciliationEvent, ReconciliationError>;
}

// ─── Handler Implementation ──────────────────────────────────────────────────

pub struct ReconciliationCommandHandler<R: SettlementBatchRepository + LedgerEntryRepository + SettlementExpectationRepository + FeeVarianceRepository> {
    repo: R,
}

impl<R: SettlementBatchRepository + LedgerEntryRepository + SettlementExpectationRepository + FeeVarianceRepository> ReconciliationCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SettlementBatchRepository + LedgerEntryRepository + SettlementExpectationRepository + FeeVarianceRepository + Send + Sync> CommandHandler for ReconciliationCommandHandler<R> {
    async fn ingest_settlement_batch(&self, cmd: IngestSettlementBatch) -> Result<IngestBatchResult, ReconciliationError> {
        // 1. Compute checksum (TODO: use SHA-256 for production)
        let checksum = content_hash(&cmd.raw_file);

        // 2. Check for duplicate (INV-06)
        if self.repo.find_batch_by_checksum(&checksum).await?.is_some() {
            return Err(ReconciliationError::DuplicateBatch(checksum));
        }

        // 3. Parse raw file (simplified: create single record from raw data)
        let batch_id = Uuid::now_v7();
        let record_id = Uuid::now_v7();

        // In production, parse based on file_format. For now, create a placeholder record.
        let record = SettlementRecord {
            record_id,
            settlement_batch_id: batch_id,
            transaction_id: format!("txn_{}", Uuid::now_v7()),
            acquirer_reference: None,
            amount_minor: 0,
            currency: "AED".into(),
            fee_minor: None,
            settlement_date: None,
            status: "pending".into(),
            match_outcome: None,
            matched_payment_intent_id: None,
        };

        let batch = SettlementBatch::new(
            batch_id,
            cmd.operator_id,
            cmd.acquirer_link_id,
            checksum,
            format!("{:?}", cmd.file_format),
            vec![record],
        );

        let event = ReconciliationEvent::SettlementBatchIngested(SettlementBatchIngested {
            settlement_batch_id: batch_id,
            operator_id: cmd.operator_id,
            acquirer_link_id: cmd.acquirer_link_id,
            total_records: batch.total_records,
            total_amount_minor: batch.total_amount_minor,
            file_format: format!("{:?}", cmd.file_format),
            occurred_at: Utc::now(),
        });

        self.repo.save_settlement_batch(&batch).await?;

        Ok(IngestBatchResult {
            settlement_batch_id: batch_id,
            total_records: batch.total_records,
            total_amount_minor: batch.total_amount_minor,
            events: vec![event],
        })
    }

    async fn process_batch_matching(&self, cmd: ProcessBatchMatching) -> Result<MatchingResult, ReconciliationError> {
        let mut batch = self.repo.load_settlement_batch(cmd.settlement_batch_id).await?
            .ok_or(ReconciliationError::NotFound(cmd.settlement_batch_id))?;

        let matcher = ReconciliationMatcher::default();
        let mut events = Vec::new();
        let mut matched = 0;
        let mut unmatched = 0;

        for record in &batch.records {
            if record.match_outcome.is_some() {
                continue; // Already matched
            }

            let result = matcher.match_record(record, &cmd.payment_intents);

            match result.outcome {
                SettlementMatchOutcome::AutoConfirmed | SettlementMatchOutcome::Matched => {
                    matched += 1;
                    events.push(ReconciliationEvent::SettlementRecordMatched(SettlementRecordMatched {
                        settlement_record_id: record.record_id,
                        settlement_batch_id: cmd.settlement_batch_id,
                        payment_intent_id: result.payment_intent_id.unwrap_or_default(),
                        acquirer_reference: record.acquirer_reference.clone().unwrap_or_default(),
                        matched_amount_minor: record.amount_minor,
                        fee_actual_minor: record.fee_minor,
                        confidence: result.confidence,
                        strategy: format!("{:?}", result.strategy),
                        occurred_at: Utc::now(),
                    }));

                    // Create ledger entries (double-entry)
                    if let Some(pi_id) = result.payment_intent_id {
                        let debit = LedgerEntry {
                            entry_id: Uuid::now_v7(),
                            transaction_id: pi_id,
                            entry_type: EntryType::Debit,
                            amount_minor: record.amount_minor,
                            currency: record.currency.clone(),
                            source_acquirer: batch.acquirer_link_id.to_string(),
                            reconciliation_batch_id: Some(cmd.settlement_batch_id),
                            reconciled: true,
                            created_at: Utc::now(),
                        };
                        let credit = LedgerEntry {
                            entry_id: Uuid::now_v7(),
                            transaction_id: pi_id,
                            entry_type: EntryType::Credit,
                            amount_minor: record.amount_minor,
                            currency: record.currency.clone(),
                            source_acquirer: batch.acquirer_link_id.to_string(),
                            reconciliation_batch_id: Some(cmd.settlement_batch_id),
                            reconciled: true,
                            created_at: Utc::now(),
                        };
                        self.repo.append_ledger_entry(&debit).await?;
                        self.repo.append_ledger_entry(&credit).await?;
                        events.push(ReconciliationEvent::LedgerEntryCreated(LedgerEntryCreated {
                            entry_id: debit.entry_id,
                            transaction_id: pi_id,
                            entry_type: "Debit".into(),
                            amount_minor: debit.amount_minor,
                            currency: debit.currency,
                            source_acquirer: batch.acquirer_link_id.to_string(),
                            occurred_at: Utc::now(),
                        }));
                    }
                }
                SettlementMatchOutcome::AmountMismatch | SettlementMatchOutcome::Unmatched | SettlementMatchOutcome::DuplicateReference => {
                    unmatched += 1;
                    events.push(ReconciliationEvent::SettlementRecordUnmatched(SettlementRecordUnmatched {
                        settlement_record_id: record.record_id,
                        settlement_batch_id: cmd.settlement_batch_id,
                        transaction_id: record.transaction_id.clone(),
                        amount_minor: record.amount_minor,
                        reason: format!("{:?}", result.outcome),
                        occurred_at: Utc::now(),
                    }));
                }
            }
        }

        batch.matched_count = matched;
        batch.unmatched_count = unmatched;
        batch.status = BatchStatus::Processed;
        batch.processed_at = Some(Utc::now());

        self.repo.save_settlement_batch(&batch).await?;

        Ok(MatchingResult {
            settlement_batch_id: cmd.settlement_batch_id,
            matched_count: matched,
            unmatched_count: unmatched,
            results: vec![],
            events,
        })
    }

    async fn resolve_exception(&self, cmd: ResolveException) -> Result<ReconciliationEvent, ReconciliationError> {
        let event = ReconciliationEvent::ReconciliationExceptionResolved(ReconciliationExceptionResolved {
            settlement_record_id: cmd.settlement_record_id,
            payment_intent_id: cmd.payment_intent_id,
            resolution: cmd.resolution,
            occurred_at: Utc::now(),
        });
        Ok(event)
    }

    async fn track_fee_variance(&self, cmd: TrackFeeVariance) -> Result<FeeVarianceResult, ReconciliationError> {
        let variance_minor = cmd.actual_fee_minor - cmd.estimated_fee_minor;
        let variance_percent = if cmd.estimated_fee_minor != 0 {
            (variance_minor as f64 / cmd.estimated_fee_minor as f64) * 100.0
        } else {
            0.0
        };
        let is_within_tolerance = variance_percent.abs() <= cmd.tolerance_threshold_percent;

        let variance_id = Uuid::now_v7();
        let status = if is_within_tolerance {
            FeeVarianceStatus::WithinTolerance
        } else {
            FeeVarianceStatus::VarianceDetected
        };

        let variance = FeeVariance {
            variance_id,
            payment_intent_id: cmd.payment_intent_id,
            acquirer_link_id: cmd.acquirer_link_id,
            estimated_fee_minor: cmd.estimated_fee_minor,
            actual_fee_minor: cmd.actual_fee_minor,
            variance_minor,
            variance_percent,
            is_within_tolerance,
            tolerance_threshold_percent: cmd.tolerance_threshold_percent,
            status: status.clone(),
            detected_at: Utc::now(),
            resolved_at: None,
            resolution_note: None,
        };

        let event = ReconciliationEvent::FeeVarianceDetected(FeeVarianceDetected {
            variance_id,
            payment_intent_id: cmd.payment_intent_id,
            acquirer_link_id: cmd.acquirer_link_id,
            estimated_fee_minor: cmd.estimated_fee_minor,
            actual_fee_minor: cmd.actual_fee_minor,
            variance_minor,
            variance_percent,
            is_within_tolerance,
            occurred_at: Utc::now(),
        });

        self.repo.save_fee_variance(&variance).await?;

        Ok(FeeVarianceResult { variance_id, is_within_tolerance, event })
    }

    async fn resolve_fee_variance(&self, cmd: ResolveFeeVarianceCmd) -> Result<ReconciliationEvent, ReconciliationError> {
        let mut variance = self.repo.load_fee_variance(cmd.variance_id).await?
            .ok_or(ReconciliationError::NotFound(cmd.variance_id))?;
        variance.status = FeeVarianceStatus::Resolved;
        variance.resolved_at = Some(Utc::now());
        variance.resolution_note = Some(cmd.resolution.clone());

        self.repo.save_fee_variance(&variance).await?;

        Ok(ReconciliationEvent::FeeVarianceResolved(FeeVarianceResolved {
            variance_id: cmd.variance_id,
            payment_intent_id: variance.payment_intent_id,
            resolution: cmd.resolution,
            occurred_at: Utc::now(),
        }))
    }

    async fn create_settlement_expectation(&self, cmd: CreateSettlementExpectation) -> Result<ReconciliationEvent, ReconciliationError> {
        let expectation_id = Uuid::now_v7();
        let settlement_cycle = cmd.settlement_cycle.clone();
        let expectation = SettlementExpectation {
            expectation_id,
            payment_intent_id: cmd.payment_intent_id,
            acquirer_link_id: cmd.acquirer_link_id,
            expected_settlement_date: cmd.expected_settlement_date,
            settlement_cycle: settlement_cycle.clone(),
            status: ExpectationStatus::Pending,
            settled_amount_minor: None,
            settled_at: None,
            created_at: Utc::now(),
        };

        let event = ReconciliationEvent::SettlementExpected(SettlementExpected {
            expectation_id,
            payment_intent_id: cmd.payment_intent_id,
            acquirer_link_id: cmd.acquirer_link_id,
            expected_settlement_date: cmd.expected_settlement_date,
            settlement_cycle: settlement_cycle,
            occurred_at: Utc::now(),
        });

        self.repo.save_settlement_expectation(&expectation).await?;

        Ok(event)
    }
}

// ─── Helper: Simple MD5-style hash (using SHA-256 for real)
fn content_hash(data: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
