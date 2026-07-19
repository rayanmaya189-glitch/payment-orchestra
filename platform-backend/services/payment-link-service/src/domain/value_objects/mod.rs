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
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "inactive" => Self::Inactive,
            "expired" => Self::Expired,
            _ => Self::Active,
        }
    }
}
