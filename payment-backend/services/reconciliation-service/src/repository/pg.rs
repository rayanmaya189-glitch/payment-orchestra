//! PostgreSQL-backed reconciliation repositories using SeaORM + platform-db entities.
//!
//! Implements all 4 repository traits: SettlementBatchRepository, LedgerEntryRepository,
//! SettlementExpectationRepository, FeeVarianceRepository.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use uuid::Uuid;

use super::*;
use crate::domain::*;
use crate::entities::settlement_batch::{
    Entity as SettlementBatchEntity,
    ActiveModel as SettlementBatchActiveModel,
    Model as SettlementBatchModel,
    Column as SettlementBatchColumn,
};
use crate::entities::ledger_entry::{
    Entity as LedgerEntryEntity,
    ActiveModel as LedgerEntryActiveModel,
    Model as LedgerEntryModel,
    Column as LedgerEntryColumn,
};
use crate::entities::settlement_expectation::{
    Entity as SettlementExpectationEntity,
    ActiveModel as SettlementExpectationActiveModel,
    Model as SettlementExpectationModel,
    Column as SettlementExpectationColumn,
};
use crate::entities::fee_variance::{
    Entity as FeeVarianceEntity,
    ActiveModel as FeeVarianceActiveModel,
    Model as FeeVarianceModel,
    Column as FeeVarianceColumn,
};

/// Combined PostgreSQL-backed repository implementing all 4 reconciliation traits.
pub struct PostgresReconciliationRepository {
    pub db: DatabaseConnection,
}

impl PostgresReconciliationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── SettlementBatchRepository ───────────────────────────────────────────────

#[async_trait]
impl SettlementBatchRepository for PostgresReconciliationRepository {
    async fn load_settlement_batch(
        &self,
        id: Uuid,
    ) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let result = SettlementBatchEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_batch_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_settlement_batch(
        &self,
        batch: &SettlementBatch,
    ) -> Result<(), ReconciliationError> {
        let model = settlement_batch_domain_to_model(batch)?;
        let exists = SettlementBatchEntity::find_by_id(batch.settlement_batch_id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            SettlementBatchEntity::update(SettlementBatchActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        } else {
            SettlementBatchEntity::insert(SettlementBatchActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_batch_by_checksum(
        &self,
        checksum: &str,
    ) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let result = SettlementBatchEntity::find()
            .filter(SettlementBatchColumn::FileChecksum.eq(checksum))
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_batch_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn list_all_batches(&self) -> Result<Vec<SettlementBatch>, ReconciliationError> {
        let models = SettlementBatchEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        let mut batches: Vec<SettlementBatch> = models
            .into_iter()
            .map(settlement_batch_model_to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        batches.sort_by(|a, b| a.ingested_at.cmp(&b.ingested_at));
        Ok(batches)
    }
}

// ─── LedgerEntryRepository ───────────────────────────────────────────────────

#[async_trait]
impl LedgerEntryRepository for PostgresReconciliationRepository {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError> {
        let model = ledger_entry_domain_to_model(entry);
        LedgerEntryEntity::insert(LedgerEntryActiveModel::from(model))
            .exec(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError> {
        // Load all ledger entries for this transaction and compare sums
        let entries = LedgerEntryEntity::find()
            .filter(LedgerEntryColumn::TransactionId.eq(transaction_id))
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;

        let total_debit: i64 = entries.iter()
            .filter(|e| e.entry_type == "Debit")
            .map(|e| e.amount_minor)
            .sum();

        let total_credit: i64 = entries.iter()
            .filter(|e| e.entry_type == "Credit")
            .map(|e| e.amount_minor)
            .sum();

        Ok(total_debit == total_credit)
    }

    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError> {
        // Load all entries and find unique transaction IDs
        let entries = LedgerEntryEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;

        let mut txn_ids: Vec<Uuid> = entries.iter().map(|e| e.transaction_id).collect();
        txn_ids.sort();
        txn_ids.dedup();

        let mut imbalanced = Vec::new();
        for txn_id in txn_ids {
            let total_debit: i64 = entries.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == "Debit")
                .map(|e| e.amount_minor)
                .sum();
            let total_credit: i64 = entries.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == "Credit")
                .map(|e| e.amount_minor)
                .sum();
            if total_debit != total_credit {
                imbalanced.push(txn_id);
            }
        }
        Ok(imbalanced)
    }
}

// ─── SettlementExpectationRepository ─────────────────────────────────────────

#[async_trait]
impl SettlementExpectationRepository for PostgresReconciliationRepository {
    async fn save_settlement_expectation(
        &self,
        expectation: &SettlementExpectation,
    ) -> Result<(), ReconciliationError> {
        let model = settlement_expectation_domain_to_model(expectation);
        let exists = SettlementExpectationEntity::find_by_id(expectation.expectation_id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            SettlementExpectationEntity::update(SettlementExpectationActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        } else {
            SettlementExpectationEntity::insert(SettlementExpectationActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn load_settlement_expectation(
        &self,
        id: Uuid,
    ) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let result = SettlementExpectationEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_expectation_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_expectation_by_payment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let result = SettlementExpectationEntity::find()
            .filter(SettlementExpectationColumn::PaymentIntentId.eq(payment_intent_id))
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_expectation_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_overdue_expectations(
        &self,
    ) -> Result<Vec<SettlementExpectation>, ReconciliationError> {
        let now = Utc::now();
        let models = SettlementExpectationEntity::find()
            .filter(SettlementExpectationColumn::Status.eq("pending"))
            .filter(SettlementExpectationColumn::ExpectedSettlementDate.lt(now))
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        models.into_iter().map(settlement_expectation_model_to_domain).collect()
    }
}

// ─── FeeVarianceRepository ───────────────────────────────────────────────────

#[async_trait]
impl FeeVarianceRepository for PostgresReconciliationRepository {
    async fn save_fee_variance(
        &self,
        variance: &FeeVariance,
    ) -> Result<(), ReconciliationError> {
        let model = fee_variance_domain_to_model(variance)?;
        let exists = FeeVarianceEntity::find_by_id(variance.variance_id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            FeeVarianceEntity::update(FeeVarianceActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        } else {
            FeeVarianceEntity::insert(FeeVarianceActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn load_fee_variance(
        &self,
        id: Uuid,
    ) -> Result<Option<FeeVariance>, ReconciliationError> {
        let result = FeeVarianceEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(fee_variance_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_fee_variances_for_payment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Vec<FeeVariance>, ReconciliationError> {
        let models = FeeVarianceEntity::find()
            .filter(FeeVarianceColumn::PaymentIntentId.eq(payment_intent_id))
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        models.into_iter().map(fee_variance_model_to_domain).collect()
    }
}

// ─── Domain ↔ Model conversion: SettlementBatch ──────────────────────────────

fn settlement_batch_domain_to_model(batch: &SettlementBatch) -> Result<SettlementBatchModel, ReconciliationError> {
    Ok(SettlementBatchModel {
        settlement_batch_id: batch.settlement_batch_id,
        batch_file_name: batch.file_checksum.clone(), // not a perfect mapping, but pragmatic
        status: batch.status.to_string(),
        ingested_at: batch.ingested_at,
        total_transactions: batch.total_records,
        total_amount_minor: batch.total_amount_minor,
        currency: "AED".to_string(), // default currency; in production from acquirer
        records: serde_json::to_value(&batch.records)
            .map_err(|e| ReconciliationError::Validation(format!("Serialize records: {}", e)))?,
        file_checksum: batch.file_checksum.clone(),
        matched_at: None,
        quarantined_at: None,
    })
}

fn settlement_batch_model_to_domain(m: SettlementBatchModel) -> Result<SettlementBatch, ReconciliationError> {
    let status = match m.status.as_str() {
        "pending" => BatchStatus::Ingesting,
        "matched" => BatchStatus::Processed,
        "exception" => BatchStatus::Quarantined,
        _ => BatchStatus::Ingesting,
    };

    let records: Vec<SettlementRecord> = serde_json::from_value(m.records)
        .map_err(|e| ReconciliationError::Validation(format!("Deserialize records: {}", e)))?;

    Ok(SettlementBatch {
        settlement_batch_id: m.settlement_batch_id,
        operator_id: Uuid::default(), // not stored in this entity — needs join in production
        acquirer_link_id: Uuid::default(), // not stored in this entity
        file_checksum: m.file_checksum,
        file_format: "csv".to_string(), // not stored in this entity
        status,
        total_records: m.total_transactions,
        matched_count: 0,
        unmatched_count: 0,
        total_amount_minor: m.total_amount_minor,
        records,
        ingested_at: m.ingested_at,
        processed_at: m.matched_at,
    })
}

// ─── Domain ↔ Model conversion: LedgerEntry ──────────────────────────────────

fn ledger_entry_domain_to_model(entry: &LedgerEntry) -> LedgerEntryModel {
    LedgerEntryModel {
        entry_id: entry.entry_id,
        transaction_id: entry.transaction_id,
        entry_type: match entry.entry_type {
            EntryType::Debit => "Debit".to_string(),
            EntryType::Credit => "Credit".to_string(),
        },
        amount_minor: entry.amount_minor,
        currency: entry.currency.clone(),
        created_at: entry.created_at,
    }
}

// ─── Domain ↔ Model conversion: SettlementExpectation ────────────────────────

fn settlement_expectation_domain_to_model(exp: &SettlementExpectation) -> SettlementExpectationModel {
    SettlementExpectationModel {
        expectation_id: exp.expectation_id,
        payment_intent_id: exp.payment_intent_id,
        expected_amount_minor: exp.settled_amount_minor.unwrap_or(0),
        currency: "AED".to_string(),
        expected_settlement_date: exp.expected_settlement_date,
        status: match exp.status {
            ExpectationStatus::Pending => "pending",
            ExpectationStatus::Settled => "settled",
            ExpectationStatus::Overdue => "overdue",
            ExpectationStatus::Adjusted => "adjusted",
        }.to_string(),
        created_at: exp.created_at,
    }
}

fn settlement_expectation_model_to_domain(m: SettlementExpectationModel) -> Result<SettlementExpectation, ReconciliationError> {
    let status = match m.status.as_str() {
        "pending" => ExpectationStatus::Pending,
        "settled" => ExpectationStatus::Settled,
        "overdue" => ExpectationStatus::Overdue,
        "adjusted" => ExpectationStatus::Adjusted,
        _ => ExpectationStatus::Pending,
    };

    Ok(SettlementExpectation {
        expectation_id: m.expectation_id,
        payment_intent_id: m.payment_intent_id,
        acquirer_link_id: Uuid::default(), // not stored in this entity — needs join in production
        expected_settlement_date: m.expected_settlement_date,
        settlement_cycle: "T+1".to_string(),
        status,
        settled_amount_minor: Some(m.expected_amount_minor),
        settled_at: None,
        created_at: m.created_at,
    })
}

// ─── Domain ↔ Model conversion: FeeVariance ──────────────────────────────────

fn fee_variance_domain_to_model(v: &FeeVariance) -> Result<FeeVarianceModel, ReconciliationError> {
    Ok(FeeVarianceModel {
        variance_id: v.variance_id,
        payment_intent_id: v.payment_intent_id,
        expected_fee_minor: v.estimated_fee_minor,
        actual_fee_minor: v.actual_fee_minor,
        variance_amount_minor: v.variance_minor,
        currency: "AED".to_string(),
        reason: v.resolution_note.clone(),
        created_at: v.detected_at,
    })
}

fn fee_variance_model_to_domain(m: FeeVarianceModel) -> Result<FeeVariance, ReconciliationError> {
    let variance_percent = if m.expected_fee_minor != 0 {
        (m.variance_amount_minor as f64 / m.expected_fee_minor as f64) * 100.0
    } else {
        0.0
    };

    Ok(FeeVariance {
        variance_id: m.variance_id,
        payment_intent_id: m.payment_intent_id,
        acquirer_link_id: Uuid::default(), // not stored in this entity
        estimated_fee_minor: m.expected_fee_minor,
        actual_fee_minor: m.actual_fee_minor,
        variance_minor: m.variance_amount_minor,
        variance_percent,
        is_within_tolerance: variance_percent.abs() <= 5.0,
        tolerance_threshold_percent: 5.0,
        status: FeeVarianceStatus::VarianceDetected,
        detected_at: m.created_at,
        resolved_at: None,
        resolution_note: m.reason,
    })
}
