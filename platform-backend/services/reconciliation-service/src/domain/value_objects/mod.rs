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

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "csv" => Ok(Self::Csv),
            "webhook" => Ok(Self::Webhook),
            "sftp" => Ok(Self::Sftp),
            "api" => Ok(Self::Api),
            _ => Err("unknown settlement format"),
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

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "ingesting" => Ok(Self::Ingesting),
            "processed" => Ok(Self::Processed),
            "quarantined" => Ok(Self::Quarantined),
            _ => Err("unknown settlement batch status"),
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

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "matched" => Ok(Self::Matched),
            "unmatched" => Ok(Self::Unmatched),
            "disputed" => Ok(Self::Disputed),
            _ => Err("unknown settlement match outcome"),
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
