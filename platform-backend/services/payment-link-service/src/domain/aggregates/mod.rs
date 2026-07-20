use chrono::{DateTime, Utc};
use uuid::Uuid;
use shared_types::Money;
use crate::domain::value_objects::PaymentLinkStatus;

#[derive(Debug, Clone)]
pub struct PaymentLink {
    pub link_id: Uuid, pub operator_id: Uuid, pub status: PaymentLinkStatus,
    pub description: String, pub merchant_name: String, pub amount: Money,
    pub max_uses: Option<i32>, pub current_uses: i32,
    pub expires_at: Option<DateTime<Utc>>, pub public_token: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
}

impl PaymentLink {
    pub fn new(operator_id: Uuid, description: String, merchant_name: String, amount: Money, max_uses: Option<i32>, expires_at: Option<DateTime<Utc>>) -> Self {
        use rand::Rng;
        let token: String = (0..32).map(|_| {
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..62);
            match idx { 0..26 => (b'a' + idx) as char, 26..52 => (b'A' + (idx - 26)) as char, _ => (b'0' + (idx - 52)) as char }
        }).collect();
        let now = Utc::now();
        Self { link_id: Uuid::now_v7(), operator_id, status: PaymentLinkStatus::Active,
            description, merchant_name, amount, max_uses, current_uses: 0,
            expires_at, public_token: token, metadata: None, created_at: now, updated_at: now }
    }

    pub fn is_valid(&self) -> bool {
        if self.status != PaymentLinkStatus::Active { return false; }
        if let Some(max) = self.max_uses { if self.current_uses >= max { return false; } }
        if let Some(exp) = self.expires_at { if Utc::now() > exp { return false; } }
        true
    }

    pub fn use_link(&mut self) -> Result<(), &'static str> {
        if !self.is_valid() { return Err("Payment link is no longer valid"); }
        self.current_uses += 1;
        self.updated_at = Utc::now();
        if let Some(max) = self.max_uses { if self.current_uses >= max { self.status = PaymentLinkStatus::UsedUp; } }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn aed(amount: i64) -> Money { Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() } }

    #[test]
    fn test_new_link() {
        let l = PaymentLink::new(Uuid::now_v7(), "Test".into(), "Merchant".into(), aed(1000), None, None);
        assert_eq!(l.status, PaymentLinkStatus::Active);
        assert_eq!(l.current_uses, 0);
        assert!(!l.public_token.is_empty());
    }

    #[test]
    fn test_use_link() {
        let mut l = PaymentLink::new(Uuid::now_v7(), "Test".into(), "Merchant".into(), aed(1000), Some(2), None);
        assert!(l.use_link().is_ok());
        assert_eq!(l.current_uses, 1);
        assert!(l.use_link().is_ok());
        assert_eq!(l.status, PaymentLinkStatus::UsedUp);
        assert!(l.use_link().is_err());
    }

    #[test]
    fn test_expired_link() {
        let mut l = PaymentLink::new(Uuid::now_v7(), "Test".into(), "Merchant".into(), aed(1000), None, Some(Utc::now() - chrono::Duration::hours(1)));
        assert!(!l.is_valid());
    }
}
