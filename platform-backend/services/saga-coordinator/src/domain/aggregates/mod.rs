use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{SagaStatus, SagaStepStatus};

/// Predefined step definitions for the payment lifecycle saga.
pub struct PaymentSagaSteps;

impl PaymentSagaSteps {
    /// The standard authorize -> capture -> settle payment lifecycle.
    pub fn payment_lifecycle() -> Vec<SagaStep> {
        vec![
            SagaStep {
                step_number: 1,
                name: "authorize".into(),
                service: "orchestration".into(),
                action: "PaymentAuthorize".into(),
                compensation_action: Some("PaymentVoid".into()),
                status: SagaStepStatus::Pending,
                error: None,
                started_at: None,
                completed_at: None,
            },
            SagaStep {
                step_number: 2,
                name: "capture".into(),
                service: "orchestration".into(),
                action: "PaymentCapture".into(),
                compensation_action: Some("PaymentRefund".into()),
                status: SagaStepStatus::Pending,
                error: None,
                started_at: None,
                completed_at: None,
            },
            SagaStep {
                step_number: 3,
                name: "settle".into(),
                service: "settlement".into(),
                action: "SettlePayment".into(),
                compensation_action: None, // Settlement reversal is handled separately
                status: SagaStepStatus::Pending,
                error: None,
                started_at: None,
                completed_at: None,
            },
        ]
    }

    /// Build custom steps from definitions.
    pub fn from_defs(defs: Vec<SagaStepDef>) -> Vec<SagaStep> {
        defs.into_iter()
            .enumerate()
            .map(|(i, d)| SagaStep {
                step_number: (i + 1) as u32,
                name: d.name,
                service: d.service,
                action: d.action,
                compensation_action: d.compensation_action,
                status: SagaStepStatus::Pending,
                error: None,
                started_at: None,
                completed_at: None,
            })
            .collect()
    }
}

/// Input definition for creating a saga step.
#[derive(Debug, Clone)]
pub struct SagaStepDef {
    pub name: String,
    pub service: String,
    pub action: String,
    pub compensation_action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SagaInstance {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub status: SagaStatus,
    pub current_step: u32,
    pub total_steps: u32,
    pub steps: Vec<SagaStep>,
    pub payload: serde_json::Value,
    pub compensation_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SagaStep {
    pub step_number: u32,
    pub name: String,
    pub service: String,
    pub action: String,
    pub compensation_action: Option<String>,
    pub status: SagaStepStatus,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl SagaInstance {
    pub fn new(saga_type: String, steps: Vec<SagaStep>, payload: serde_json::Value) -> Self {
        let total = steps.len() as u32;
        let now = Utc::now();
        Self {
            saga_id: Uuid::now_v7(),
            saga_type,
            status: SagaStatus::Running,
            current_step: 1,
            total_steps: total,
            steps,
            payload,
            compensation_data: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Advance to the next step and mark it as running.
    /// Returns the step being started, or None if all steps are done.
    pub fn advance_step(&mut self) -> Result<SagaStep, SagaError> {
        if self.status != SagaStatus::Running {
            return Err(SagaError::InvalidState {
                current: self.status.as_str().to_string(),
                attempted: "advance".into(),
            });
        }
        if self.current_step > self.total_steps {
            return Err(SagaError::SagaAlreadyCompleted);
        }

        let idx = (self.current_step - 1) as usize;
        self.steps[idx].status = SagaStepStatus::Running;
        self.steps[idx].started_at = Some(Utc::now());
        self.updated_at = Utc::now();

        Ok(self.steps[idx].clone())
    }

    /// Mark the current step as completed and advance the cursor.
    pub fn complete_step(&mut self) -> Result<(), SagaError> {
        if self.status != SagaStatus::Running {
            return Err(SagaError::InvalidState {
                current: self.status.as_str().to_string(),
                attempted: "complete".into(),
            });
        }

        let idx = (self.current_step - 1) as usize;
        self.steps[idx].status = SagaStepStatus::Completed;
        self.steps[idx].completed_at = Some(Utc::now());
        self.current_step += 1;
        self.updated_at = Utc::now();

        if self.current_step > self.total_steps {
            self.status = SagaStatus::Completed;
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Mark the current step as failed. The saga transitions to Failed.
    pub fn fail_step(&mut self, error: &str) -> Result<(), SagaError> {
        if self.status != SagaStatus::Running {
            return Err(SagaError::InvalidState {
                current: self.status.as_str().to_string(),
                attempted: "fail".into(),
            });
        }

        let idx = (self.current_step - 1) as usize;
        self.steps[idx].status = SagaStepStatus::Failed;
        self.steps[idx].error = Some(error.to_string());
        self.steps[idx].completed_at = Some(Utc::now());
        self.status = SagaStatus::Failed;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Transition to compensating and return the steps that need compensation
    /// (completed steps with compensation actions, in reverse order).
    pub fn begin_compensation(&mut self) -> Result<Vec<SagaStep>, SagaError> {
        if self.status != SagaStatus::Failed {
            return Err(SagaError::InvalidState {
                current: self.status.as_str().to_string(),
                attempted: "compensate".into(),
            });
        }

        self.status = SagaStatus::Compensating;
        self.updated_at = Utc::now();

        let to_compensate: Vec<SagaStep> = self
            .steps
            .iter()
            .rev()
            .filter(|s| s.status == SagaStepStatus::Completed && s.compensation_action.is_some())
            .cloned()
            .collect();

        Ok(to_compensate)
    }

    /// Mark a single step as compensated (called after its compensation action succeeds).
    pub fn mark_step_compensated(&mut self, step_number: u32) -> Result<(), SagaError> {
        if self.status != SagaStatus::Compensating {
            return Err(SagaError::InvalidState {
                current: self.status.as_str().to_string(),
                attempted: "mark_compensated".into(),
            });
        }

        let idx = (step_number - 1) as usize;
        if idx >= self.steps.len() {
            return Err(SagaError::InvalidStepNumber(step_number));
        }

        self.steps[idx].status = SagaStepStatus::Compensated;
        self.updated_at = Utc::now();

        // Check if all compensable steps are done
        let all_done = self.steps.iter().all(|s| {
            s.status == SagaStepStatus::Compensated
                || s.status == SagaStepStatus::Failed
                || s.status == SagaStepStatus::Pending
                || (s.status == SagaStepStatus::Completed && s.compensation_action.is_none())
        });

        if all_done {
            self.status = SagaStatus::Compensated;
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Record compensation data (e.g. reasons, transaction IDs from compensations).
    pub fn set_compensation_data(&mut self, data: serde_json::Value) {
        self.compensation_data = Some(data);
        self.updated_at = Utc::now();
    }

    /// Get the current step being executed.
    pub fn current_step_ref(&self) -> Option<&SagaStep> {
        if self.current_step >= 1 && self.current_step <= self.total_steps {
            Some(&self.steps[(self.current_step - 1) as usize])
        } else {
            None
        }
    }

    /// Check if this saga type is a recognized payment lifecycle.
    pub fn is_payment_lifecycle(&self) -> bool {
        self.saga_type == "payment_lifecycle"
    }
}

/// Errors that can occur during saga state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaError {
    InvalidState { current: String, attempted: String },
    SagaAlreadyCompleted,
    InvalidStepNumber(u32),
    CompensationFailed { step_number: u32, error: String },
}

impl std::fmt::Display for SagaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidState { current, attempted } => {
                write!(f, "Cannot {attempted} saga in {current} state")
            }
            Self::SagaAlreadyCompleted => write!(f, "Saga is already completed"),
            Self::InvalidStepNumber(n) => write!(f, "Invalid step number: {n}"),
            Self::CompensationFailed { step_number, error } => {
                write!(f, "Compensation failed for step {step_number}: {error}")
            }
        }
    }
}

impl std::error::Error for SagaError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payment_steps() -> Vec<SagaStep> {
        PaymentSagaSteps::payment_lifecycle()
    }

    fn make_simple_steps() -> Vec<SagaStep> {
        vec![
            SagaStep {
                step_number: 1,
                name: "charge".into(),
                service: "orchestration".into(),
                action: "authorize".into(),
                compensation_action: Some("void".into()),
                status: SagaStepStatus::Pending,
                error: None,
                started_at: None,
                completed_at: None,
            },
            SagaStep {
                step_number: 2,
                name: "notify".into(),
                service: "notification".into(),
                action: "send".into(),
                compensation_action: None,
                status: SagaStepStatus::Pending,
                error: None,
                started_at: None,
                completed_at: None,
            },
        ]
    }

    #[test]
    fn test_new_saga() {
        let s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        assert_eq!(s.status, SagaStatus::Running);
        assert_eq!(s.total_steps, 2);
        assert_eq!(s.current_step, 1);
    }

    #[test]
    fn test_payment_lifecycle_steps() {
        let steps = make_payment_steps();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].name, "authorize");
        assert_eq!(steps[1].name, "capture");
        assert_eq!(steps[2].name, "settle");
        assert_eq!(steps[0].compensation_action.as_deref(), Some("PaymentVoid"));
        assert_eq!(steps[1].compensation_action.as_deref(), Some("PaymentRefund"));
        assert!(steps[2].compensation_action.is_none());
    }

    #[test]
    fn test_advance_step_ok() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        let step = s.advance_step().unwrap();
        assert_eq!(step.name, "charge");
        assert_eq!(s.steps[0].status, SagaStepStatus::Running);
        assert!(s.steps[0].started_at.is_some());
    }

    #[test]
    fn test_advance_step_already_completed() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.status = SagaStatus::Completed;
        let err = s.advance_step().unwrap_err();
        assert_eq!(
            err,
            SagaError::InvalidState {
                current: "completed".into(),
                attempted: "advance".into()
            }
        );
    }

    #[test]
    fn test_complete_step_advances_cursor() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.advance_step().unwrap();
        s.complete_step().unwrap();
        assert_eq!(s.current_step, 2);
        assert_eq!(s.status, SagaStatus::Running);
    }

    #[test]
    fn test_complete_last_step_marks_saga_completed() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.advance_step().unwrap();
        s.complete_step().unwrap();
        s.advance_step().unwrap();
        s.complete_step().unwrap();
        assert_eq!(s.status, SagaStatus::Completed);
        assert!(s.completed_at.is_some());
    }

    #[test]
    fn test_fail_step() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.advance_step().unwrap();
        s.fail_step("timeout").unwrap();
        assert_eq!(s.status, SagaStatus::Failed);
        assert_eq!(s.steps[0].error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_fail_step_wrong_state() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.status = SagaStatus::Completed;
        let err = s.fail_step("err").unwrap_err();
        assert!(matches!(err, SagaError::InvalidState { .. }));
    }

    #[test]
    fn test_compensation_flow() {
        let mut s = SagaInstance::new("payment".into(), make_payment_steps(), serde_json::json!({}));

        // Complete step 1 (authorize)
        s.advance_step().unwrap();
        s.complete_step().unwrap();

        // Complete step 2 (capture)
        s.advance_step().unwrap();
        s.complete_step().unwrap();

        // Step 3 (settle) fails
        s.advance_step().unwrap();
        s.fail_step("settlement failed").unwrap();
        assert_eq!(s.status, SagaStatus::Failed);

        // Begin compensation
        let to_compensate = s.begin_compensation().unwrap();
        assert_eq!(s.status, SagaStatus::Compensating);
        // Should compensate capture (step 2) then authorize (step 1), in reverse
        assert_eq!(to_compensate.len(), 2);
        assert_eq!(to_compensate[0].name, "capture");
        assert_eq!(to_compensate[1].name, "authorize");

        // Mark each compensated
        s.mark_step_compensated(2).unwrap();
        assert_eq!(s.status, SagaStatus::Compensating); // not done yet
        s.mark_step_compensated(1).unwrap();
        assert_eq!(s.status, SagaStatus::Compensated);
    }

    #[test]
    fn test_compensation_no_action_on_steps_without_compensation() {
        let mut s = SagaInstance::new(
            "mixed".into(),
            vec![
                SagaStep {
                    step_number: 1,
                    name: "charge".into(),
                    service: "orchestration".into(),
                    action: "authorize".into(),
                    compensation_action: Some("void".into()),
                    status: SagaStepStatus::Pending,
                    error: None,
                    started_at: None,
                    completed_at: None,
                },
                SagaStep {
                    step_number: 2,
                    name: "notify".into(),
                    service: "notification".into(),
                    action: "send".into(),
                    compensation_action: None, // no compensation
                    status: SagaStepStatus::Pending,
                    error: None,
                    started_at: None,
                    completed_at: None,
                },
            ],
            serde_json::json!({}),
        );

        s.advance_step().unwrap();
        s.complete_step().unwrap();
        s.advance_step().unwrap();
        s.fail_step("timeout").unwrap();

        let to_compensate = s.begin_compensation().unwrap();
        // Only step 1 has compensation_action
        assert_eq!(to_compensate.len(), 1);
        assert_eq!(to_compensate[0].name, "charge");

        s.mark_step_compensated(1).unwrap();
        assert_eq!(s.status, SagaStatus::Compensated);
    }

    #[test]
    fn test_compensation_wrong_state() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        let err = s.begin_compensation().unwrap_err();
        assert_eq!(
            err,
            SagaError::InvalidState {
                current: "running".into(),
                attempted: "compensate".into()
            }
        );
    }

    #[test]
    fn test_mark_step_compensated_invalid_step() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.advance_step().unwrap();
        s.fail_step("err").unwrap();
        s.begin_compensation().unwrap();
        let err = s.mark_step_compensated(99).unwrap_err();
        assert_eq!(err, SagaError::InvalidStepNumber(99));
    }

    #[test]
    fn test_complete_step_wrong_state() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        s.status = SagaStatus::Failed;
        let err = s.complete_step().unwrap_err();
        assert!(matches!(err, SagaError::InvalidState { .. }));
    }

    #[test]
    fn test_current_step_ref() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        let step = s.current_step_ref().unwrap();
        assert_eq!(step.name, "charge");

        // After completing all steps
        s.advance_step().unwrap();
        s.complete_step().unwrap();
        s.advance_step().unwrap();
        s.complete_step().unwrap();
        assert!(s.current_step_ref().is_none());
    }

    #[test]
    fn test_is_payment_lifecycle() {
        let s = SagaInstance::new(
            "payment_lifecycle".into(),
            make_payment_steps(),
            serde_json::json!({}),
        );
        assert!(s.is_payment_lifecycle());

        let s2 = SagaInstance::new("custom".into(), make_simple_steps(), serde_json::json!({}));
        assert!(!s2.is_payment_lifecycle());
    }

    #[test]
    fn test_set_compensation_data() {
        let mut s = SagaInstance::new("payment".into(), make_simple_steps(), serde_json::json!({}));
        let data = serde_json::json!({"reason": "timeout", "compensation_tx_ids": ["tx1"]});
        s.set_compensation_data(data.clone());
        assert_eq!(s.compensation_data, Some(data));
    }

    #[test]
    fn test_full_payment_lifecycle() {
        let mut s = SagaInstance::new(
            "payment_lifecycle".into(),
            make_payment_steps(),
            serde_json::json!({"amount": 5000, "currency": "AED"}),
        );

        // Step 1: Authorize
        let step = s.advance_step().unwrap();
        assert_eq!(step.action, "PaymentAuthorize");
        s.complete_step().unwrap();
        assert_eq!(s.current_step, 2);

        // Step 2: Capture
        let step = s.advance_step().unwrap();
        assert_eq!(step.action, "PaymentCapture");
        s.complete_step().unwrap();
        assert_eq!(s.current_step, 3);

        // Step 3: Settle
        let step = s.advance_step().unwrap();
        assert_eq!(step.action, "SettlePayment");
        s.complete_step().unwrap();
        assert_eq!(s.status, SagaStatus::Completed);
    }

    #[test]
    fn test_from_defs() {
        let defs = vec![
            SagaStepDef {
                name: "step1".into(),
                service: "svc1".into(),
                action: "act1".into(),
                compensation_action: Some("comp1".into()),
            },
            SagaStepDef {
                name: "step2".into(),
                service: "svc2".into(),
                action: "act2".into(),
                compensation_action: None,
            },
        ];
        let steps = PaymentSagaSteps::from_defs(defs);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].step_number, 1);
        assert_eq!(steps[1].step_number, 2);
    }

    #[test]
    fn test_saga_error_display() {
        let e = SagaError::InvalidState {
            current: "completed".into(),
            attempted: "advance".into(),
        };
        assert_eq!(e.to_string(), "Cannot advance saga in completed state");

        let e = SagaError::SagaAlreadyCompleted;
        assert_eq!(e.to_string(), "Saga is already completed");

        let e = SagaError::InvalidStepNumber(42);
        assert_eq!(e.to_string(), "Invalid step number: 42");

        let e = SagaError::CompensationFailed {
            step_number: 1,
            error: "network error".into(),
        };
        assert_eq!(e.to_string(), "Compensation failed for step 1: network error");
    }
}
