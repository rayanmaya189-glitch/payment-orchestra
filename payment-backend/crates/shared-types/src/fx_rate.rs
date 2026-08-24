use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::money::CurrencyCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRate {
    pub source_currency: CurrencyCode,
    pub target_currency: CurrencyCode,
    pub rate: String,
    pub rate_minor_units: i64,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxQuote {
    pub quote_id: Uuid,
    pub source_amount: super::money::Money,
    pub target_amount: super::money::Money,
    pub rate: FxRate,
    pub fee: Option<super::money::Money>,
}
