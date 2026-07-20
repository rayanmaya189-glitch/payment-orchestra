use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::PaymentLinkStatus;
use shared_types::Money;

#[derive(Debug, Clone)]
pub struct PaymentLink {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub status: PaymentLinkStatus,
    pub amount: Money,
    pub description: String,
    pub merchant_name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub created_at: DateTime<Utc>,
}

impl PaymentLink {
    pub fn new(operator_id: Uuid, amount: Money, description: String, merchant_name: String) -> Self {
        Self {
            link_id: Uuid::now_v7(),
            operator_id,
            status: PaymentLinkStatus::Active,
            amount,
            description,
            merchant_name,
            expires_at: None,
            max_uses: None,
            current_uses: 0,
            created_at: Utc::now(),
        }
    }

    pub fn is_usable(&self) -> bool {
        if self.status != PaymentLinkStatus::Active { return false; }
        if let Some(exp) = self.expires_at {
            if Utc::now() > exp { return false; }
        }
        if let Some(max) = self.max_uses {
            if self.current_uses >= max { return false; }
        }
        true
    }

    pub fn record_use(&mut self) {
        self.current_uses += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() }
    }

    #[test]
    fn test_new_link_is_active() {
        let link = PaymentLink::new(Uuid::now_v7(), aed(10000), "Test".into(), "Merchant".into());
        assert_eq!(link.status, PaymentLinkStatus::Active);
        assert_eq!(link.current_uses, 0);
        assert!(link.is_usable());
    }

    #[test]
    fn test_link_not_usable_when_inactive() {
        let mut link = PaymentLink::new(Uuid::now_v7(), aed(10000), "Test".into(), "Merchant".into());
        link.status = PaymentLinkStatus::Inactive;
        assert!(!link.is_usable());
    }

    #[test]
    fn test_link_not_usable_when_expired() {
        let mut link = PaymentLink::new(Uuid::now_v7(), aed(10000), "Test".into(), "Merchant".into());
        link.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(!link.is_usable());
    }

    #[test]
    fn test_link_not_usable_when_max_uses_reached() {
        let mut link = PaymentLink::new(Uuid::now_v7(), aed(10000), "Test".into(), "Merchant".into());
        link.max_uses = Some(2);
        link.record_use();
        link.record_use();
        assert!(!link.is_usable());
    }

    #[test]
    fn test_link_usable_before_max_uses() {
        let mut link = PaymentLink::new(Uuid::now_v7(), aed(10000), "Test".into(), "Merchant".into());
        link.max_uses = Some(3);
        link.record_use();
        link.record_use();
        assert!(link.is_usable());
    }

    #[test]
    fn test_record_use_increments() {
        let mut link = PaymentLink::new(Uuid::now_v7(), aed(10000), "Test".into(), "Merchant".into());
        assert_eq!(link.current_uses, 0);
        link.record_use();
        assert_eq!(link.current_uses, 1);
        link.record_use();
        assert_eq!(link.current_uses, 2);
    }
}
