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
