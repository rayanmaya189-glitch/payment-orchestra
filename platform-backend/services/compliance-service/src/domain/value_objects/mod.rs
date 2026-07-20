#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KybStatus {
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Suspended,
}

impl KybStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::UnderReview => "under_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Suspended => "suspended",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "under_review" => Self::UnderReview,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            "suspended" => Self::Suspended,
            _ => Self::Submitted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KybDecision {
    Approved,
    Rejected,
    Suspended,
}

impl KybDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Suspended => "suspended",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "rejected" => Self::Rejected,
            "suspended" => Self::Suspended,
            _ => Self::Approved,
        }
    }
}
