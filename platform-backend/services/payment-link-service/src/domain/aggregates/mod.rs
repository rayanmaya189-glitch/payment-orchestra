use chrono::{DateTime, Utc};
use rand::Rng;
use uuid::Uuid;
use shared_types::Money;

use crate::domain::value_objects::PaymentLinkStatus;

/// URL-safe alphabet for token generation (RFC 4648 §5 without padding).
const TOKEN_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

#[derive(Debug, Clone)]
pub struct PaymentLink {
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub status: PaymentLinkStatus,
    pub description: String,
    pub merchant_name: String,
    pub amount: Money,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub public_token: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentLink {
    pub fn new(
        operator_id: Uuid,
        description: String,
        merchant_name: String,
        amount: Money,
        max_uses: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        let token = Self::generate_token(32);
        let now = Utc::now();
        Self {
            link_id: Uuid::now_v7(),
            operator_id,
            status: PaymentLinkStatus::Active,
            description,
            merchant_name,
            amount,
            max_uses,
            current_uses: 0,
            expires_at,
            public_token: token,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a cryptographically random URL-safe token.
    fn generate_token(length: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..TOKEN_CHARSET.len());
                TOKEN_CHARSET[idx] as char
            })
            .collect()
    }

    pub fn is_valid(&self) -> bool {
        if self.status != PaymentLinkStatus::Active {
            return false;
        }
        if let Some(max) = self.max_uses {
            if self.current_uses >= max {
                return false;
            }
        }
        if let Some(exp) = self.expires_at {
            if Utc::now() > exp {
                return false;
            }
        }
        true
    }

    pub fn use_link(&mut self) -> Result<(), &'static str> {
        if !self.is_valid() {
            return Err("Payment link is no longer valid");
        }
        self.current_uses += 1;
        self.updated_at = Utc::now();
        if let Some(max) = self.max_uses {
            if self.current_uses >= max {
                self.status = PaymentLinkStatus::UsedUp;
            }
        }
        Ok(())
    }

    /// Deactivate the payment link.
    pub fn deactivate(&mut self) -> Result<(), &'static str> {
        if self.status != PaymentLinkStatus::Active {
            return Err("Only active payment links can be deactivated");
        }
        self.status = PaymentLinkStatus::Deactivated;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the link metadata.
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = Some(metadata);
        self.updated_at = Utc::now();
    }

    /// Update the link description.
    pub fn update_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Check if the link has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            Utc::now() > exp
        } else {
            false
        }
    }

    /// Check if the link has reached its max uses.
    pub fn is_maxed_out(&self) -> bool {
        if let Some(max) = self.max_uses {
            self.current_uses >= max
        } else {
            false
        }
    }

    /// Get the remaining uses (None if unlimited).
    pub fn remaining_uses(&self) -> Option<i32> {
        self.max_uses.map(|max| max.saturating_sub(self.current_uses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money {
            amount_minor_units: amount,
            currency: shared_types::CurrencyCode::new("AED").unwrap(),
        }
    }

    fn test_link() -> PaymentLink {
        PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            None,
            None,
        )
    }

    #[test]
    fn test_new_link() {
        let l = test_link();
        assert_eq!(l.status, PaymentLinkStatus::Active);
        assert_eq!(l.current_uses, 0);
        assert!(!l.public_token.is_empty());
        assert_eq!(l.public_token.len(), 32);
    }

    #[test]
    fn test_token_is_url_safe() {
        let l = test_link();
        for c in l.public_token.chars() {
            assert!(
                c.is_ascii_alphanumeric(),
                "Token contains non-URL-safe char: {c}"
            );
        }
    }

    #[test]
    fn test_use_link() {
        let mut l = PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            Some(2),
            None,
        );
        assert!(l.use_link().is_ok());
        assert_eq!(l.current_uses, 1);
        assert!(l.use_link().is_ok());
        assert_eq!(l.status, PaymentLinkStatus::UsedUp);
        assert!(l.use_link().is_err());
    }

    #[test]
    fn test_expired_link() {
        let l = PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            None,
            Some(Utc::now() - chrono::Duration::hours(1)),
        );
        assert!(!l.is_valid());
        assert!(l.is_expired());
    }

    #[test]
    fn test_deactivate() {
        let mut l = test_link();
        assert!(l.deactivate().is_ok());
        assert_eq!(l.status, PaymentLinkStatus::Deactivated);
        assert!(!l.is_valid());
    }

    #[test]
    fn test_deactivate_already_deactivated() {
        let mut l = test_link();
        l.status = PaymentLinkStatus::Deactivated;
        assert_eq!(
            l.deactivate(),
            Err("Only active payment links can be deactivated")
        );
    }

    #[test]
    fn test_deactivate_used_up() {
        let mut l = PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            Some(1),
            None,
        );
        l.use_link().unwrap();
        assert_eq!(l.status, PaymentLinkStatus::UsedUp);
        assert_eq!(
            l.deactivate(),
            Err("Only active payment links can be deactivated")
        );
    }

    #[test]
    fn test_update_metadata() {
        let mut l = test_link();
        assert!(l.metadata.is_none());
        l.update_metadata(serde_json::json!({"key": "value"}));
        assert_eq!(
            l.metadata,
            Some(serde_json::json!({"key": "value"}))
        );
    }

    #[test]
    fn test_update_description() {
        let mut l = test_link();
        l.update_description("New description".to_string());
        assert_eq!(l.description, "New description");
    }

    #[test]
    fn test_is_maxed_out() {
        let mut l = PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            Some(2),
            None,
        );
        assert!(!l.is_maxed_out());
        l.use_link().unwrap();
        assert!(!l.is_maxed_out());
        l.use_link().unwrap();
        assert!(l.is_maxed_out());
    }

    #[test]
    fn test_remaining_uses() {
        let mut l = PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            Some(5),
            None,
        );
        assert_eq!(l.remaining_uses(), Some(5));
        l.use_link().unwrap();
        assert_eq!(l.remaining_uses(), Some(4));
        l.use_link().unwrap();
        l.use_link().unwrap();
        assert_eq!(l.remaining_uses(), Some(2));
    }

    #[test]
    fn test_remaining_uses_unlimited() {
        let l = test_link();
        assert_eq!(l.remaining_uses(), None);
    }

    #[test]
    fn test_link_with_expiry() {
        let l = PaymentLink::new(
            Uuid::now_v7(),
            "Test".into(),
            "Merchant".into(),
            aed(1000),
            None,
            Some(Utc::now() + chrono::Duration::hours(24)),
        );
        assert!(l.is_valid());
        assert!(!l.is_expired());
    }

    #[test]
    fn test_unique_tokens() {
        let l1 = test_link();
        let l2 = test_link();
        // Extremely unlikely to collide with 32-char alphanumeric tokens
        assert_ne!(l1.public_token, l2.public_token);
    }

    #[test]
    fn test_use_increments_updated_at() {
        let mut l = test_link();
        let before = l.updated_at;
        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));
        l.use_link().unwrap();
        assert!(l.updated_at >= before);
    }
}
