#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementFormat {
    Csv,
    Webhook,
    Sftp,
    Api,
}

impl SettlementFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Webhook => "webhook",
            Self::Sftp => "sftp",
            Self::Api => "api",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "csv" => Self::Csv,
            "webhook" => Self::Webhook,
            "sftp" => Self::Sftp,
            "api" => Self::Api,
            _ => Self::Api,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementBatchStatus {
    Ingesting,
    Processed,
    Quarantined,
}

impl SettlementBatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ingesting => "ingesting",
            Self::Processed => "processed",
            Self::Quarantined => "quarantined",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "ingesting" => Self::Ingesting,
            "processed" => Self::Processed,
            "quarantined" => Self::Quarantined,
            _ => Self::Ingesting,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementMatchOutcome {
    Matched,
    Unmatched,
    Disputed,
}

impl SettlementMatchOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Unmatched => "unmatched",
            Self::Disputed => "disputed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "matched" => Self::Matched,
            "unmatched" => Self::Unmatched,
            "disputed" => Self::Disputed,
            _ => Self::Unmatched,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettlementRecord {
    pub acquirer_reference: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub settled_at: chrono::DateTime<chrono::Utc>,
    pub fee_minor_units: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub payment_intent_id: Option<uuid::Uuid>,
    pub confidence: f64,
    pub outcome: SettlementMatchOutcome,
    pub auto_confirm: bool,
}
