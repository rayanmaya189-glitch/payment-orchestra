//! Analytics Service command types

use uuid::Uuid;

pub struct IngestAnalyticsEvent {
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
}
