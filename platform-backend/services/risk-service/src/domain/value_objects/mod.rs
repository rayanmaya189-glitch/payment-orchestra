#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskDecision { Allow, Review, Decline }
impl RiskDecision {
    pub fn as_str(&self) -> &'static str { match self { Self::Allow => "allow", Self::Review => "review", Self::Decline => "decline" } }
    pub fn from_str(s: &str) -> Self { match s { "review" => Self::Review, "decline" => Self::Decline, _ => Self::Allow } }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RiskFactor {
    pub factor: String,
    pub score: f64,
    pub weight: f64,
    pub description: String,
}
