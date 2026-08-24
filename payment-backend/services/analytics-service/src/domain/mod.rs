//! Analytics Service domain model — BC-15
//!
//! Pure query-side read model. Consumes domain events into ClickHouse
//! (simulated with in-memory store for Phase 1) and provides 8 analytics
//! read endpoints.

pub mod advanced_analytics;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AnalyticsEvent — raw event stored in ClickHouse (in-memory for Phase 1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_id: Uuid,
    pub event_type: String,
    pub payment_intent_id: Option<Uuid>,
    pub operator_id: Option<Uuid>,
    pub acquirer_id: Option<String>,
    pub card_scheme: Option<String>,
    pub currency: Option<String>,
    pub amount_minor_units: Option<i64>,
    pub decline_reason: Option<String>,
    pub latency_ms: Option<u32>,
    pub acquirer_fee: Option<i64>,
    pub chargeback_amount: Option<i64>,
    pub chargeback_reason: Option<String>,
    pub fraud_score: Option<f64>,
    pub bin: Option<String>,
    pub country_code: Option<String>,
    pub merchant_id: Option<String>,
    pub failover_routed: Option<bool>,
    pub occurred_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Query result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRateRow {
    pub hour: DateTime<Utc>,
    pub acquirer_id: String,
    pub card_scheme: String,
    pub approved_count: u64,
    pub declined_count: u64,
    pub total_count: u64,
    pub auth_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclineReasonRow {
    pub decline_reason: String,
    pub count: u64,
    pub percentage_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementStatusRow {
    pub status: String,
    pub count: u64,
    pub total_amount_minor_units: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeAnalysisRow {
    pub acquirer_id: String,
    pub total_fees_minor_units: i64,
    pub transaction_count: u64,
    pub avg_fee_per_transaction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackTrendRow {
    pub card_scheme: String,
    pub chargeback_count: u64,
    pub transaction_count: u64,
    pub chargeback_rate_pct: f64,
    pub period_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeComplianceRow {
    pub card_scheme: String,
    pub threshold_name: String,
    pub current_rate_pct: f64,
    pub threshold_pct: f64,
    pub is_breaching: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudAnalysisRow {
    pub dimension: String,
    pub dimension_value: String,
    pub transaction_count: u64,
    pub fraud_count: u64,
    pub fraud_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecoveryRow {
    pub period: String,
    pub failover_count: u64,
    pub recovered_amount_minor_units: i64,
    pub estimated_savings_minor_units: i64,
    pub currency: String,
}

// ---------------------------------------------------------------------------
// Query filters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeFilter {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRateQuery {
    pub date_range: DateRangeFilter,
    pub acquirer_ids: Option<Vec<String>>,
    pub card_schemes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclineReasonQuery {
    pub date_range: DateRangeFilter,
    pub acquirer_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeAnalysisQuery {
    pub date_range: DateRangeFilter,
    pub acquirer_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackTrendQuery {
    pub period_days: u32,
    pub card_schemes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudAnalysisQuery {
    pub date_range: DateRangeFilter,
    pub dimension: FraudDimension,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FraudDimension {
    Bin,
    Country,
    AmountRange,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum AnalyticsError {
    #[error("Analytics unavailable: {0}")]
    Unavailable(String),
    #[error("Stale data: last ingestion was {0} seconds ago")]
    StaleData(i64),
    #[error("Invalid date range: {0}")]
    InvalidDateRange(String),
    #[error("Query timeout: {0}")]
    QueryTimeout(String),
    #[error("Event not found: {0}")]
    EventNotFound(Uuid),
    #[error("Invalid event type: {0}")]
    InvalidEventType(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
}

// ---------------------------------------------------------------------------
// Event types consumed by the analytics service
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalyticsEventType {
    PaymentAuthorized,
    PaymentCaptured,
    PaymentFailed,
    PaymentRefunded,
    PaymentVoided,
    ChargebackReceived,
    ChargebackResolved,
    FeeRecorded,
    SettlementMatched,
    SettlementUnmatched,
    RoutingDecision,
    RiskAssessment,
}

impl AnalyticsEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PaymentAuthorized => "PaymentAuthorized",
            Self::PaymentCaptured => "PaymentCaptured",
            Self::PaymentFailed => "PaymentFailed",
            Self::PaymentRefunded => "PaymentRefunded",
            Self::PaymentVoided => "PaymentVoided",
            Self::ChargebackReceived => "ChargebackReceived",
            Self::ChargebackResolved => "ChargebackResolved",
            Self::FeeRecorded => "FeeRecorded",
            Self::SettlementMatched => "SettlementMatched",
            Self::SettlementUnmatched => "SettlementUnmatched",
            Self::RoutingDecision => "RoutingDecision",
            Self::RiskAssessment => "RiskAssessment",
        }
    }
}
