//! SagaStatus — full state machine for saga lifecycle.

use serde::{Deserialize, Serialize};

// created → running → completed
//                   → compensating → compensated
//                   → failed → requires_manual_intervention

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaStatus {
    /// Saga created, not yet started.
    Created,
    /// Saga is actively executing steps.
    Running,
    /// All steps completed successfully.
    Completed,
    /// Saga is compensating (undoing) completed steps.
    Compensating,
    /// All steps have been compensated.
    Compensated,
    /// Saga execution failed (non-compensated).
    Failed,
    /// Saga requires manual intervention (after failed compensation).
    RequiresManualIntervention,
}

impl SagaStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use SagaStatus::*;
        matches!(
            (self, target),
            (Created, Running)
                | (Running, Completed)
                | (Running, Compensating)
                | (Running, Failed)
                | (Compensating, Compensated)
                | (Compensating, RequiresManualIntervention)
                | (Failed, RequiresManualIntervention)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Compensated | Self::RequiresManualIntervention)
    }
}

impl std::fmt::Display for SagaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Compensating => write!(f, "compensating"),
            Self::Compensated => write!(f, "compensated"),
            Self::Failed => write!(f, "failed"),
            Self::RequiresManualIntervention => write!(f, "requires_manual_intervention"),
        }
    }
}
