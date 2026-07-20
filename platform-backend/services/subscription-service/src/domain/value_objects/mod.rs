#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionStatus { Active, Paused, Cancelled, PastDue }
impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Active => "active", Self::Paused => "paused", Self::Cancelled => "cancelled", Self::PastDue => "past_due" } }
    pub fn from_str(s: &str) -> Result<Self, &'static str> { match s { "active" => Ok(Self::Active), "paused" => Ok(Self::Paused), "cancelled" => Ok(Self::Cancelled), "past_due" => Ok(Self::PastDue), _ => Err("unknown subscription status") } }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingInterval { Monthly, Quarterly, Yearly }
impl BillingInterval {
    pub fn as_str(&self) -> &'static str { match self { Self::Monthly => "monthly", Self::Quarterly => "quarterly", Self::Yearly => "yearly" } }
    pub fn from_str(s: &str) -> Result<Self, &'static str> { match s { "monthly" => Ok(Self::Monthly), "quarterly" => Ok(Self::Quarterly), "yearly" => Ok(Self::Yearly), _ => Err("unknown billing interval") } }
}
