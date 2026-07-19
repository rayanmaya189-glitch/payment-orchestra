use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskDecision { Approve, Review, Reject }
impl RiskDecision {
    pub fn as_str(&self) -> &'static str { match self { Self::Approve => "approve", Self::Review => "review", Self::Reject => "reject" } }
    pub fn from_str(s: &str) -> Self { match s { "approve" => Self::Approve, "review" => Self::Review, "reject" => Self::Reject, _ => Self::Review } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor { pub factor: String, pub score: f64, pub weight: f64, pub description: String }
