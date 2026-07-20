#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentLinkStatus {
    Active,
    Inactive,
    Expired,
}

impl PaymentLinkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Expired => "expired",
        }
    }
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "expired" => Ok(Self::Expired),
            _ => Err("unknown payment link status"),
        }
    }
}
