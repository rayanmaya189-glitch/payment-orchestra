use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SagaStatus {
    Running,
    Completed,
    Failed,
    Compensating,
    Compensated,
}

impl SagaStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Compensating => "compensating",
            Self::Compensated => "compensated",
        }
    }

    /// Valid forward transitions from this status.
    pub fn can_transition_to(&self, target: &SagaStatus) -> bool {
        matches!(
            (self, target),
            (Self::Running, Self::Completed)
                | (Self::Running, Self::Failed)
                | (Self::Running, Self::Compensating)
                | (Self::Failed, Self::Compensating)
                | (Self::Compensating, Self::Compensated)
                | (Self::Compensating, Self::Failed)
        )
    }
}

impl fmt::Display for SagaStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SagaStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "compensating" => Ok(Self::Compensating),
            "compensated" => Ok(Self::Compensated),
            other => Err(format!("unknown saga status: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SagaStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Compensated,
}

impl SagaStepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Compensated => "compensated",
        }
    }
}

impl fmt::Display for SagaStepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SagaStepStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "compensated" => Ok(Self::Compensated),
            other => Err(format!("unknown saga step status: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saga_status_roundtrip() {
        for status in [
            SagaStatus::Running,
            SagaStatus::Completed,
            SagaStatus::Failed,
            SagaStatus::Compensating,
            SagaStatus::Compensated,
        ] {
            let s = status.as_str();
            let parsed: SagaStatus = s.parse().unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_saga_status_invalid_parse() {
        assert!("bogus".parse::<SagaStatus>().is_err());
    }

    #[test]
    fn test_valid_transitions() {
        assert!(SagaStatus::Running.can_transition_to(&SagaStatus::Completed));
        assert!(SagaStatus::Running.can_transition_to(&SagaStatus::Failed));
        assert!(SagaStatus::Failed.can_transition_to(&SagaStatus::Compensating));
        assert!(SagaStatus::Compensating.can_transition_to(&SagaStatus::Compensated));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!SagaStatus::Completed.can_transition_to(&SagaStatus::Running));
        assert!(!SagaStatus::Compensated.can_transition_to(&SagaStatus::Running));
        assert!(!SagaStatus::Running.can_transition_to(&SagaStatus::Compensated));
    }

    #[test]
    fn test_step_status_roundtrip() {
        for status in [
            SagaStepStatus::Pending,
            SagaStepStatus::Running,
            SagaStepStatus::Completed,
            SagaStepStatus::Failed,
            SagaStepStatus::Compensated,
        ] {
            let s = status.as_str();
            let parsed: SagaStepStatus = s.parse().unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_saga_status_display() {
        assert_eq!(SagaStatus::Running.to_string(), "running");
    }

    #[test]
    fn test_step_status_display() {
        assert_eq!(SagaStepStatus::Pending.to_string(), "pending");
    }
}
