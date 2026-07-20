#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisputeStatus { Opened, UnderReview, EvidenceSubmitted, Resolved }
impl DisputeStatus { pub fn as_str(&self) -> &'static str { match self { Self::Opened => "opened", Self::UnderReview => "under_review", Self::EvidenceSubmitted => "evidence_submitted", Self::Resolved => "resolved" } } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisputeDecision { Won, Lost, Expired }
impl DisputeDecision { pub fn as_str(&self) -> &'static str { match self { Self::Won => "won", Self::Lost => "lost", Self::Expired => "expired" } } }
