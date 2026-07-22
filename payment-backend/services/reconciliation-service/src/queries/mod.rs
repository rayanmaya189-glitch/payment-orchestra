//! Query handlers for reconciliation-service read models.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

#[derive(Debug, Clone)]
pub struct GetSettlementBatchQuery {
    pub settlement_batch_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct GetUnmatchedRecordsQuery {
    pub settlement_batch_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct GetFeeVarianceQuery {
    pub payment_intent_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CheckLedgerBalanceQuery {
    pub transaction_id: Uuid,
}

#[async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_settlement_batch(&self, query: GetSettlementBatchQuery) -> Result<Option<SettlementBatch>, ReconciliationError>;
    async fn get_unmatched_records(&self, query: GetUnmatchedRecordsQuery) -> Result<Vec<SettlementRecord>, ReconciliationError>;
    async fn get_fee_variances(&self, query: GetFeeVarianceQuery) -> Result<Vec<FeeVariance>, ReconciliationError>;
    async fn check_ledger_balance(&self, query: CheckLedgerBalanceQuery) -> Result<bool, ReconciliationError>;
    async fn find_overdue_expectations(&self) -> Result<Vec<SettlementExpectation>, ReconciliationError>;
}

pub struct ReconciliationQueryHandler<R: SettlementBatchRepository + LedgerEntryRepository + SettlementExpectationRepository + FeeVarianceRepository> {
    repo: R,
}

impl<R: SettlementBatchRepository + LedgerEntryRepository + SettlementExpectationRepository + FeeVarianceRepository> ReconciliationQueryHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SettlementBatchRepository + LedgerEntryRepository + SettlementExpectationRepository + FeeVarianceRepository + Send + Sync> QueryHandler for ReconciliationQueryHandler<R> {
    async fn get_settlement_batch(&self, query: GetSettlementBatchQuery) -> Result<Option<SettlementBatch>, ReconciliationError> {
        self.repo.load_settlement_batch(query.settlement_batch_id).await
    }

    async fn get_unmatched_records(&self, query: GetUnmatchedRecordsQuery) -> Result<Vec<SettlementRecord>, ReconciliationError> {
        let batch = self.repo.load_settlement_batch(query.settlement_batch_id).await?;
        match batch {
            Some(b) => Ok(b.records.into_iter()
                .filter(|r| matches!(r.match_outcome, Some(SettlementMatchOutcome::Unmatched) | None))
                .collect()),
            None => Ok(vec![]),
        }
    }

    async fn get_fee_variances(&self, query: GetFeeVarianceQuery) -> Result<Vec<FeeVariance>, ReconciliationError> {
        self.repo.find_fee_variances_for_payment(query.payment_intent_id).await
    }

    async fn check_ledger_balance(&self, query: CheckLedgerBalanceQuery) -> Result<bool, ReconciliationError> {
        self.repo.verify_balance(query.transaction_id).await
    }

    async fn find_overdue_expectations(&self) -> Result<Vec<SettlementExpectation>, ReconciliationError> {
        self.repo.find_overdue_expectations().await
    }
}
