use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::money::Money;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeVariance {
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub estimated_fee: Money,
    pub actual_fee: Money,
    pub variance_amount: Money,
    pub variance_percent: f64,
    pub is_within_tolerance: bool,
    pub detected_at: DateTime<Utc>,
}

impl FeeVariance {
    pub fn new(
        payment_intent_id: Uuid,
        acquirer_link_id: Uuid,
        estimated_fee: Money,
        actual_fee: Money,
        tolerance_percent: f64,
    ) -> Result<Self, &'static str> {
        if estimated_fee.currency != actual_fee.currency {
            return Err("Fee currency mismatch");
        }
        let variance_amount = actual_fee.checked_sub(&estimated_fee)
            .map_err(|_| "Fee variance calculation error")?;
        let variance_percent = if estimated_fee.amount_minor_units != 0 {
            (actual_fee.amount_minor_units - estimated_fee.amount_minor_units) as f64
                / estimated_fee.amount_minor_units as f64 * 100.0
        } else {
            0.0
        };
        let is_within_tolerance = variance_percent.abs() <= tolerance_percent;
        Ok(Self {
            payment_intent_id,
            acquirer_link_id,
            estimated_fee,
            actual_fee,
            variance_amount,
            variance_percent,
            is_within_tolerance,
            detected_at: Utc::now(),
        })
    }
}
