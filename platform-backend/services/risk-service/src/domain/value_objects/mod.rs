#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskDecision { Approve, Review, Reject }
impl RiskDecision {
    pub fn as_str(&self) -> &'static str { match self { Self::Approve => "approve", Self::Review => "review", Self::Reject => "reject" } }
    pub fn from_str(s: &str) -> Result<Self, &'static str> { match s { "approve" => Ok(Self::Approve), "review" => Ok(Self::Review), "reject" => Ok(Self::Reject), _ => Err("unknown risk decision") } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor { pub factor: String, pub score: f64, pub weight: f64, pub description: String }
