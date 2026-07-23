//! AnalyticsEvent entity — `analytics_events` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "analytics_events")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub event_id: Uuid,
    pub event_type: String,
    #[sea_orm(column_type = "Uuid")]
    pub payment_intent_id: Option<Uuid>,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Option<Uuid>,
    pub acquirer_id: Option<String>,
    pub card_scheme: Option<String>,
    pub currency: Option<String>,
    pub amount_minor_units: Option<i64>,
    pub decline_reason: Option<String>,
    pub latency_ms: Option<i32>,
    pub acquirer_fee: Option<i64>,
    pub chargeback_amount: Option<i64>,
    pub chargeback_reason: Option<String>,
    pub fraud_score: Option<f64>,
    pub bin: Option<String>,
    pub country_code: Option<String>,
    pub merchant_id: Option<String>,
    pub failover_routed: Option<bool>,
    pub occurred_at: DateTimeUtc,
    pub ingested_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
