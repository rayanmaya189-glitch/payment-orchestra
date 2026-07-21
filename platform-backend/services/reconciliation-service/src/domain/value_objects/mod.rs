use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettlementStatus {
    Pending,
    Polled,
    Matched,
    Settled,
    Exception,
}

impl SettlementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Polled => "polled",
            Self::Matched => "matched",
            Self::Settled => "settled",
            Self::Exception => "exception",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "polled" => Self::Polled,
            "matched" => Self::Matched,
            "settled" => Self::Settled,
            "exception" => Self::Exception,
            _ => Self::Pending,
        }
    }
}

impl fmt::Display for SettlementStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Match outcome for a single settlement record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchOutcome {
    /// Exact match on acquirer_reference with matching amount.
    Matched,
    /// Acquirer_reference matches but amounts differ.
    AmountMismatch,
    /// No matching acquirer_reference found.
    Unmatched,
    /// Multiple internal records share the same acquirer_reference.
    DuplicateReference,
    /// Matched with fee discrepancy.
    FeeDiscrepancy,
}

impl MatchOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::AmountMismatch => "amount_mismatch",
            Self::Unmatched => "unmatched",
            Self::DuplicateReference => "duplicate_reference",
            Self::FeeDiscrepancy => "fee_discrepancy",
        }
    }
}

impl fmt::Display for MatchOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A parsed settlement record from the connector file.
#[derive(Debug, Clone)]
pub struct SettlementRecord {
    pub acquirer_reference: String,
    pub amount: shared_types::Money,
    pub settled_at: chrono::DateTime<chrono::Utc>,
    pub fee: Option<shared_types::Money>,
    pub connector_id: String,
    pub status: String,
}

/// An internal ledger entry representing a payment in our system.
#[derive(Debug, Clone)]
pub struct LedgerEntryRecord {
    pub entry_id: uuid::Uuid,
    pub transaction_id: uuid::Uuid,
    pub acquirer_reference: String,
    pub amount: shared_types::Money,
    pub fee: Option<shared_types::Money>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of matching a single settlement record against internal records.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub settlement: SettlementRecord,
    pub outcome: MatchOutcome,
    pub matched_entry: Option<LedgerEntryRecord>,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_status_roundtrip() {
        for variant in [
            SettlementStatus::Pending,
            SettlementStatus::Polled,
            SettlementStatus::Matched,
            SettlementStatus::Settled,
            SettlementStatus::Exception,
        ] {
            let s = variant.as_str();
            assert_eq!(SettlementStatus::from_str(s), variant);
        }
    }

    #[test]
    fn test_settlement_status_display() {
        assert_eq!(SettlementStatus::Matched.to_string(), "matched");
    }

    #[test]
    fn test_match_outcome_display() {
        assert_eq!(MatchOutcome::Matched.to_string(), "matched");
        assert_eq!(MatchOutcome::Unmatched.to_string(), "unmatched");
        assert_eq!(MatchOutcome::AmountMismatch.to_string(), "amount_mismatch");
    }

    #[test]
    fn test_settlement_status_unknown_defaults_to_pending() {
        assert_eq!(SettlementStatus::from_str("bogus"), SettlementStatus::Pending);
    }
}
