//! Query type definitions for reconciliation-service.

use uuid::Uuid;

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
