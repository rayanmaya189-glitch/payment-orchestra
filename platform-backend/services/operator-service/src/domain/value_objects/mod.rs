use platform_error::ValidationError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorStatus {
    Pending,
    ActiveUnverified,
    ActiveVerified,
    Suspended,
    ExpiredUnverified,
}

impl OperatorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::ActiveUnverified => "active_unverified",
            Self::ActiveVerified => "active_verified",
            Self::Suspended => "suspended",
            Self::ExpiredUnverified => "expired_unverified",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "active_unverified" => Some(Self::ActiveUnverified),
            "active_verified" => Some(Self::ActiveVerified),
            "suspended" => Some(Self::Suspended),
            "expired_unverified" => Some(Self::ExpiredUnverified),
            _ => None,
        }
    }
}

impl std::fmt::Display for OperatorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeLicenseNo(String);

impl TradeLicenseNo {
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        if value.is_empty() || value.len() > 64 {
            return Err(ValidationError::MissingField("trade_license_no".into()));
        }
        // Basic format validation: alphanumeric + hyphens
        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::MissingField(
                "trade_license_no format invalid".into(),
            ));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TradeLicenseNo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
