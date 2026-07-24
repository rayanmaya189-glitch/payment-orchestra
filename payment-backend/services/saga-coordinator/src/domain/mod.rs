//! Saga Coordinator domain model — BC-17
//!
//! Durable state machine for multi-step, cross-aggregate workflows
//! with automatic compensation on failure.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// SagaStatus — full state machine
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// SagaType — known saga definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaType {
    PaymentLifecycle,
    SubscriptionRenewal,
    ReconciliationResolution,
    InvoicePayment,
}

impl std::fmt::Display for SagaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaymentLifecycle => write!(f, "payment_lifecycle"),
            Self::SubscriptionRenewal => write!(f, "subscription_renewal"),
            Self::ReconciliationResolution => write!(f, "reconciliation_resolution"),
            Self::InvoicePayment => write!(f, "invoice_payment"),
        }
    }
}

// ---------------------------------------------------------------------------
// SagaStep — a single step within a saga
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub step_id: Uuid,
    pub step_name: String,
    pub action: String,       // e.g., "authorize", "capture", "void"
    pub compensation_action: String, // e.g., "void", "refund"
    pub status: StepStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Executing,
    Succeeded,
    Failed,
    Compensated,
}

// ---------------------------------------------------------------------------
// SagaInstance aggregate
// ---------------------------------------------------------------------------

/// Core SagaInstance aggregate root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaInstance {
    pub saga_id: Uuid,
    pub saga_type: SagaType,
    pub aggregate_id: Uuid,
    pub status: SagaStatus,
    pub steps: Vec<SagaStep>,
    pub compensation_attempts: i32,
    pub max_compensation_retries: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline_at: Option<DateTime<Utc>>,
}

impl SagaInstance {
    /// Max compensation retries before manual intervention.
    pub const MAX_COMPENSATION_RETRIES: i32 = 3;

    /// Default step timeout: 30 seconds.
    pub const DEFAULT_STEP_TIMEOUT_SECONDS: i64 = 30;

    /// Create a new saga in `Created` status.
    pub fn new(
        saga_type: SagaType,
        aggregate_id: Uuid,
        steps: Vec<SagaStep>,
    ) -> Result<Self, SagaError> {
        if steps.is_empty() {
            return Err(SagaError::NoStepsDefined);
        }

        let now = Utc::now();
        Ok(Self {
            saga_id: Uuid::now_v7(),
            saga_type,
            aggregate_id,
            status: SagaStatus::Created,
            steps,
            compensation_attempts: 0,
            max_compensation_retries: Self::MAX_COMPENSATION_RETRIES,
            created_at: now,
            updated_at: now,
            deadline_at: Some(now + Duration::seconds(Self::DEFAULT_STEP_TIMEOUT_SECONDS)),
        })
    }

    /// Start the saga: transition from Created → Running.
    pub fn start(&mut self) -> Result<(), SagaError> {
        if !self.status.can_transition_to(&SagaStatus::Running) {
            return Err(SagaError::InvalidTransition);
        }
        self.status = SagaStatus::Running;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get the next pending step to execute.
    pub fn next_pending_step(&self) -> Option<&SagaStep> {
        self.steps.iter().find(|s| s.status == StepStatus::Pending)
    }

    /// Get the next pending step index.
    pub fn next_pending_step_index(&self) -> Option<usize> {
        self.steps.iter().position(|s| s.status == StepStatus::Pending)
    }

    /// Mark a step as executing.
    pub fn start_step(&mut self, step_index: usize) -> Result<(), SagaError> {
        let step = self
            .steps
            .get_mut(step_index)
            .ok_or(SagaError::StepNotFound)?;

        if step.status != StepStatus::Pending {
            return Err(SagaError::InvalidStepTransition);
        }

        step.status = StepStatus::Executing;
        step.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark a step as succeeded. If all steps done → Completed.
    pub fn complete_step(&mut self, step_index: usize, output: String) -> Result<(), SagaError> {
        let step = self
            .steps
            .get_mut(step_index)
            .ok_or(SagaError::StepNotFound)?;

        if step.status != StepStatus::Executing {
            return Err(SagaError::InvalidStepTransition);
        }

        step.status = StepStatus::Succeeded;
        step.output = Some(output);
        step.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();

        // Check if all steps are done
        if self.steps.iter().all(|s| s.status == StepStatus::Succeeded || s.status == StepStatus::Compensated) {
            self.status = SagaStatus::Completed;
        }

        Ok(())
    }

    /// Mark a step as failed → begin compensation.
    pub fn fail_step(&mut self, step_index: usize, error: String) -> Result<(), SagaError> {
        let step = self
            .steps
            .get_mut(step_index)
            .ok_or(SagaError::StepNotFound)?;

        if step.status != StepStatus::Executing {
            return Err(SagaError::InvalidStepTransition);
        }

        step.status = StepStatus::Failed;
        step.error = Some(error);
        self.updated_at = Utc::now();

        // Begin compensation (reverse-order)
        self.status = SagaStatus::Compensating;
        Ok(())
    }

    /// Compensate the next succeeded step (reverse order).
    /// SAGA-006: Reverse-order compensation (stack-based).
    pub fn compensate_next_step(&mut self) -> Result<Option<usize>, SagaError> {
        // Find the last succeeded step
        let compensate_index = self
            .steps
            .iter()
            .rposition(|s| s.status == StepStatus::Succeeded);

        if let Some(idx) = compensate_index {
            self.steps[idx].status = StepStatus::Compensated;
            self.compensation_attempts += 1;
            self.updated_at = Utc::now();

            // Check if all steps are handled (pending steps never reached are ok)
            if self.steps.iter().all(|s| {
                s.status == StepStatus::Pending
                    || s.status == StepStatus::Compensated
                    || s.status == StepStatus::Failed
            }) {
                self.status = SagaStatus::Compensated;
            }

            Ok(Some(idx))
        } else {
            // All succeeded steps compensated
            self.status = SagaStatus::Compensated;
            Ok(None)
        }
    }

    /// Mark compensation as failed → requires manual intervention.
    pub fn fail_compensation(&mut self, error: String) -> Result<(), SagaError> {
        if self.compensation_attempts >= self.max_compensation_retries {
            self.status = SagaStatus::RequiresManualIntervention;
            self.updated_at = Utc::now();
            return Err(SagaError::CompensationFailed(error));
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark saga as failed (non-compensated).
    pub fn mark_failed(&mut self, error: String) -> Result<(), SagaError> {
        if !self.status.can_transition_to(&SagaStatus::Failed) {
            return Err(SagaError::InvalidTransition);
        }
        self.status = SagaStatus::Failed;
        self.updated_at = Utc::now();
        // Add error to the last executing step
        if let Some(step) = self.steps.iter_mut().find(|s| s.status == StepStatus::Executing) {
            step.error = Some(error);
        }
        Ok(())
    }

    /// Check if the saga has timed out.
    pub fn is_timed_out(&self) -> bool {
        if let Some(deadline) = self.deadline_at {
            self.status == SagaStatus::Running && Utc::now() > deadline
        } else {
            false
        }
    }

    /// Get default steps for payment lifecycle saga.
    pub fn payment_lifecycle_steps() -> Vec<SagaStep> {
        vec![
            Self::make_step("authorize", "authorize", "void"),
            Self::make_step("capture", "capture", "void"),
            Self::make_step("settle", "settle", "refund"),
            Self::make_step("reconcile", "reconcile", "unreconcile"),
        ]
    }

    fn make_step(name: &str, action: &str, compensation: &str) -> SagaStep {
        SagaStep {
            step_id: Uuid::now_v7(),
            step_name: name.into(),
            action: action.into(),
            compensation_action: compensation.into(),
            status: StepStatus::Pending,
            output: None,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum SagaError {
    #[error("Saga not found: {0}")]
    NotFound(Uuid),
    #[error("Saga already completed")]
    AlreadyCompleted,
    #[error("Invalid saga status transition")]
    InvalidTransition,
    #[error("Invalid step transition")]
    InvalidStepTransition,
    #[error("No steps defined for saga")]
    NoStepsDefined,
    #[error("Step not found")]
    StepNotFound,
    #[error("Saga step execution failed: {0}")]
    StepFailed(String),
    #[error("Compensation failed after retries: {0}")]
    CompensationFailed(String),
    #[error("Saga timed out")]
    Timeout,
}
