use chrono::{DateTime, Utc};
use uuid::Uuid;
use shared_types::Money;
use crate::domain::value_objects::SettlementStatus;

#[derive(Debug, Clone)]
pub struct SettlementBatch {
    pub batch_id: Uuid, pub operator_id: Uuid, pub connector_id: String,
    pub status: SettlementStatus, pub total_amount: Money,
    pub total_records: i32, pub matched_count: i32, pub unmatched_count: i32, pub exception_count: i32,
    pub period_start: String, pub period_end: String,
    pub exceptions: Option<serde_json::Value>,
    pub polled_at: Option<DateTime<Utc>>, pub matched_at: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
}

impl SettlementBatch {
    pub fn new(operator_id: Uuid, connector_id: String, period_start: String, period_end: String) -> Self {
        let now = Utc::now();
        Self { batch_id: Uuid::now_v7(), operator_id, connector_id, status: SettlementStatus::Pending,
            total_amount: Money { amount_minor_units: 0, currency: shared_types::CurrencyCode::new("AED").unwrap() },
            total_records: 0, matched_count: 0, unmatched_count: 0, exception_count: 0,
            period_start, period_end, exceptions: None, polled_at: None, matched_at: None,
            settled_at: None, created_at: now, updated_at: now }
    }

    pub fn mark_polled(&mut self, records: i32, total_amount: i64) {
        self.status = SettlementStatus::Polled;
        self.total_records = records;
        self.total_amount.amount_minor_units = total_amount;
        self.polled_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_matched(&mut self, matched: i32, unmatched: i32) {
        self.status = SettlementStatus::Matched;
        self.matched_count = matched;
        self.unmatched_count = unmatched;
        self.matched_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_settled(&mut self) {
        self.status = SettlementStatus::Settled;
        self.settled_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_batch() {
        let b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        assert_eq!(b.status, SettlementStatus::Pending);
    }
    #[test]
    fn test_mark_polled() {
        let mut b = SettlementBatch::new(Uuid::now_v7(), "ni".into(), "2024-01-01".into(), "2024-01-31".into());
        b.mark_polled(10, 100000);
        assert_eq!(b.status, SettlementStatus::Polled);
        assert_eq!(b.total_records, 10);
    }
}
