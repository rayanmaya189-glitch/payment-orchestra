use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeStatus { Open, UnderReview, Resolved, Closed }
impl DisputeStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Open => "open", Self::UnderReview => "under_review", Self::Resolved => "resolved", Self::Closed => "closed" } }
    pub fn from_str(s: &str) -> Self { match s { "open" => Self::Open, "under_review" => Self::UnderReview, "resolved" => Self::Resolved, "closed" => Self::Closed, _ => Self::Open } }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeReason { Fraudulent, NotReceived, NotAsDescribed, Duplicate, Other(String) }
impl DisputeReason {
    pub fn as_str(&self) -> &'static str { match self { Self::Fraudulent => "fraudulent", Self::NotReceived => "not_received", Self::NotAsDescribed => "not_as_described", Self::Duplicate => "duplicate", Self::Other(_) => "other" } }
}
