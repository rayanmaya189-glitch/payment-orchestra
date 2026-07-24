use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingStatus {
    Draft,
    CredentialsSubmitted,
    Testing,
    Active,
    Deactivated,
    Revoked,
}

impl OnboardingStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use OnboardingStatus::*;
        matches!(
            (self, target),
            (Draft, CredentialsSubmitted)
                | (CredentialsSubmitted, Testing)
                | (Testing, Active)
                | (Testing, CredentialsSubmitted)
                | (Active, Deactivated)
                | (Active, Revoked)
                | (Deactivated, Active)
                | (Deactivated, Revoked)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Revoked)
    }
}

impl std::fmt::Display for OnboardingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::CredentialsSubmitted => write!(f, "credentials_submitted"),
            Self::Testing => write!(f, "testing"),
            Self::Active => write!(f, "active"),
            Self::Deactivated => write!(f, "deactivated"),
            Self::Revoked => write!(f, "revoked"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Degraded,
    Down,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Down => write!(f, "down"),
        }
    }
}
