use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionStatus { Active, Paused, Cancelled, PastDue }
impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Active => "active", Self::Paused => "paused", Self::Cancelled => "cancelled", Self::PastDue => "past_due" } }
    pub fn from_str(s: &str) -> Self { match s { "active" => Self::Active, "paused" => Self::Paused, "cancelled" => Self::Cancelled, "past_due" => Self::PastDue, _ => Self::Active } }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingInterval { Monthly, Quarterly, Yearly }
impl BillingInterval {
    pub fn as_str(&self) -> &'static str { match self { Self::Monthly => "monthly", Self::Quarterly => "quarterly", Self::Yearly => "yearly" } }
    pub fn from_str(s: &str) -> Self { match s { "monthly" => Self::Monthly, "quarterly" => Self::Quarterly, "yearly" => Self::Yearly, _ => Self::Monthly } }
}
