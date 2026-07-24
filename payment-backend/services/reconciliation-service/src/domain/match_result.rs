//! Settlement matching result types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchResult {
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Option<Uuid>,
    pub confidence: f64,
    pub strategy: MatchStrategy,
    pub outcome: SettlementMatchOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementMatchOutcome {
    AutoConfirmed,
    Matched,
    AmountMismatch,
    Unmatched,
    DuplicateReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchStrategy {
    Exact,
    Fuzzy,
    AiAssisted,
}

/// Lightweight payment intent reference for matching
#[derive(Debug, Clone)]
pub struct PaymentIntentRef {
    pub payment_intent_id: Uuid,
    pub acquirer_reference: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
}
