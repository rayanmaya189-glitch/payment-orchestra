use platform_error::{PlatformError, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisputeStatus {
    Opened,
    UnderReview,
    EvidenceSubmitted,
    Resolved,
}

impl DisputeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::UnderReview => "under_review",
            Self::EvidenceSubmitted => "evidence_submitted",
            Self::Resolved => "resolved",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "opened" => Some(Self::Opened),
            "under_review" => Some(Self::UnderReview),
            "evidence_submitted" => Some(Self::EvidenceSubmitted),
            "resolved" => Some(Self::Resolved),
            _ => None,
        }
    }

    /// Validate a state transition. Returns Err if the transition is illegal.
    pub fn validate_transition(&self, target: &DisputeStatus) -> Result<(), PlatformError> {
        let valid = match (self, target) {
            (Self::Opened, Self::UnderReview) => true,
            (Self::Opened, Self::EvidenceSubmitted) => true,
            (Self::UnderReview, Self::EvidenceSubmitted) => true,
            (Self::EvidenceSubmitted, Self::Resolved) => true,
            (Self::Opened, Self::Resolved) => true, // expired shortcut
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(PlatformError::Validation(
                ValidationError::InvalidStateTransition {
                    from: self.as_str().to_string(),
                    command: target.as_str().to_string(),
                },
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisputeDecision {
    Won,
    Lost,
    Expired,
}

impl DisputeDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Won => "won",
            Self::Lost => "lost",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "won" => Some(Self::Won),
            "lost" => Some(Self::Lost),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_from_str() {
        assert_eq!(DisputeStatus::from_str("opened"), Some(DisputeStatus::Opened));
        assert_eq!(DisputeStatus::from_str("resolved"), Some(DisputeStatus::Resolved));
        assert_eq!(DisputeStatus::from_str("bogus"), None);
    }

    #[test]
    fn test_decision_from_str() {
        assert_eq!(DisputeDecision::from_str("won"), Some(DisputeDecision::Won));
        assert_eq!(DisputeDecision::from_str("lost"), Some(DisputeDecision::Lost));
        assert_eq!(DisputeDecision::from_str("expired"), Some(DisputeDecision::Expired));
        assert_eq!(DisputeDecision::from_str("maybe"), None);
    }

    #[test]
    fn test_valid_transitions() {
        assert!(DisputeStatus::Opened.validate_transition(&DisputeStatus::UnderReview).is_ok());
        assert!(DisputeStatus::Opened.validate_transition(&DisputeStatus::EvidenceSubmitted).is_ok());
        assert!(DisputeStatus::UnderReview.validate_transition(&DisputeStatus::EvidenceSubmitted).is_ok());
        assert!(DisputeStatus::EvidenceSubmitted.validate_transition(&DisputeStatus::Resolved).is_ok());
        assert!(DisputeStatus::Opened.validate_transition(&DisputeStatus::Resolved).is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(DisputeStatus::Resolved.validate_transition(&DisputeStatus::Opened).is_err());
        assert!(DisputeStatus::EvidenceSubmitted.validate_transition(&DisputeStatus::Opened).is_err());
        assert!(DisputeStatus::UnderReview.validate_transition(&DisputeStatus::Opened).is_err());
    }
}
