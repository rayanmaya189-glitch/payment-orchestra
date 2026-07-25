//! Chargeback lifecycle status with valid state transitions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargebackStatus {
    /// Chargeback received from acquirer.
    Received,
    /// Case is under manual review.
    UnderReview,
    /// Representment evidence has been submitted to acquirer.
    RepresentmentSubmitted,
    /// Chargeback was won (funds returned to merchant).
    Won,
    /// Chargeback was lost (stand).
    Lost,
    /// Merchant accepted the chargeback.
    Accepted,
    /// Case escalated to scheme arbitration.
    Escalated,
}

impl ChargebackStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use ChargebackStatus::*;
        match (self, target) {
            (Received, UnderReview) => true,
            (Received, RepresentmentSubmitted) => true,
            (Received, Accepted) => true,
            (UnderReview, RepresentmentSubmitted) => true,
            (UnderReview, Accepted) => true,
            (RepresentmentSubmitted, Won) => true,
            (RepresentmentSubmitted, Lost) => true,
            (RepresentmentSubmitted, Escalated) => true,
            (Won, _) | (Lost, _) | (Accepted, _) | (Escalated, _) => false,
            _ => false,
        }
    }

    /// Returns true if this is a terminal (resolved) status.
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Won | Self::Lost | Self::Accepted)
    }
}

impl std::str::FromStr for ChargebackStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "received" => Ok(ChargebackStatus::Received),
            "under_review" => Ok(ChargebackStatus::UnderReview),
            "representment_submitted" => Ok(ChargebackStatus::RepresentmentSubmitted),
            "won" => Ok(ChargebackStatus::Won),
            "lost" => Ok(ChargebackStatus::Lost),
            "accepted" => Ok(ChargebackStatus::Accepted),
            "escalated" => Ok(ChargebackStatus::Escalated),
            _ => Err(format!("Invalid chargeback status: {}", s)),
        }
    }
}

impl std::fmt::Display for ChargebackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Received => write!(f, "received"),
            Self::UnderReview => write!(f, "under_review"),
            Self::RepresentmentSubmitted => write!(f, "representment_submitted"),
            Self::Won => write!(f, "won"),
            Self::Lost => write!(f, "lost"),
            Self::Accepted => write!(f, "accepted"),
            Self::Escalated => write!(f, "escalated"),
        }
    }
}
