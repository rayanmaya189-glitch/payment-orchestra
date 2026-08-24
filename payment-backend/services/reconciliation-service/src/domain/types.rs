//! Small shared enums and newtypes for reconciliation domain.

use serde::{Deserialize, Serialize};

/// SHA-256 checksum of settlement file content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementFileChecksum(pub String);

/// Settlement file format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettlementFormat {
    Webhook,
    PollingApi,
    Sftp,
    Csv,
    ScannedDocument,
}

/// Entry type for double-entry ledger
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryType {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchStatus {
    Ingesting,
    Processed,
    Quarantined,
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ingesting => write!(f, "pending"),
            Self::Processed => write!(f, "matched"),
            Self::Quarantined => write!(f, "exception"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpectationStatus {
    Pending,
    Settled,
    Overdue,
    Adjusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeeVarianceStatus {
    WithinTolerance,
    VarianceDetected,
    Disputed,
    Resolved,
}
