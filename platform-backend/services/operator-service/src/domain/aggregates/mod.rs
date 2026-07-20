use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{OperatorStatus, TradeLicenseNo};

#[derive(Debug, Clone)]
pub struct Operator {
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: TradeLicenseNo,
    pub country: String,
    pub status: OperatorStatus,
    pub subdomain: String,
    pub email: String,
    pub provisioned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Operator {
    pub fn new(
        legal_name: String,
        trade_license_no: TradeLicenseNo,
        country: String,
        email: String,
        subdomain: String,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            legal_name,
            trade_license_no,
            country,
            status: OperatorStatus::Pending,
            subdomain,
            email,
            provisioned_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn verify_email(&mut self) -> Result<(), platform_error::PlatformError> {
        if self.status != OperatorStatus::Pending {
            return Err(platform_error::PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: self.status.as_str().to_string(),
                    command: "VerifyEmail".to_string(),
                },
            ));
        }
        self.status = OperatorStatus::ActiveUnverified;
        Ok(())
    }

    pub fn can_process_live_transactions(&self) -> bool {
        self.status == OperatorStatus::ActiveVerified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_operator_is_pending() {
        let o = Operator::new("Test Corp".into(), TradeLicenseNo::new("TL-12345").unwrap(), "AE".into(), "test@corp.com".into(), "test-corp".into());
        assert_eq!(o.status, OperatorStatus::Pending);
        assert!(o.provisioned_at.is_none());
    }

    #[test]
    fn test_verify_email_transitions_to_active_unverified() {
        let mut o = Operator::new("Test Corp".into(), TradeLicenseNo::new("TL-12345").unwrap(), "AE".into(), "test@corp.com".into(), "test-corp".into());
        o.verify_email().unwrap();
        assert_eq!(o.status, OperatorStatus::ActiveUnverified);
    }

    #[test]
    fn test_verify_email_fails_if_not_pending() {
        let mut o = Operator::new("Test Corp".into(), TradeLicenseNo::new("TL-12345").unwrap(), "AE".into(), "test@corp.com".into(), "test-corp".into());
        o.verify_email().unwrap();
        let result = o.verify_email();
        assert!(result.is_err());
    }

    #[test]
    fn test_can_process_live_only_when_verified() {
        let mut o = Operator::new("Test Corp".into(), TradeLicenseNo::new("TL-12345").unwrap(), "AE".into(), "test@corp.com".into(), "test-corp".into());
        assert!(!o.can_process_live_transactions());
        o.status = OperatorStatus::ActiveVerified;
        assert!(o.can_process_live_transactions());
    }

    #[test]
    fn test_operator_status_values() {
        assert_eq!(OperatorStatus::Pending.as_str(), "pending");
        assert_eq!(OperatorStatus::ActiveUnverified.as_str(), "active_unverified");
        assert_eq!(OperatorStatus::ActiveVerified.as_str(), "active_verified");
        assert_eq!(OperatorStatus::Suspended.as_str(), "suspended");
        assert_eq!(OperatorStatus::ExpiredUnverified.as_str(), "expired_unverified");
    }
}
