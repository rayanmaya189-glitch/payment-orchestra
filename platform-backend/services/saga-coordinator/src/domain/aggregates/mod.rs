use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{SagaStatus, SagaStepStatus, SagaType};

/// Saga instance — orchestrates a cross-service workflow.
#[derive(Debug, Clone)]
pub struct SagaInstance {
    pub saga_id: Uuid,
    pub saga_type: SagaType,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub status: SagaStatus,
    pub current_step: u32,
    pub total_steps: u32,
    pub steps: Vec<SagaStep>,
    pub compensation_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl SagaInstance {
    pub fn new(saga_type: SagaType, aggregate_id: Uuid, aggregate_type: String, steps: Vec<SagaStep>) -> Self {
        let total_steps = steps.len() as u32;
        let now = Utc::now();
        Self {
            saga_id: Uuid::now_v7(),
            saga_type,
            aggregate_id,
            aggregate_type,
            status: SagaStatus::Pending,
            current_step: 0,
            total_steps,
            steps,
            compensation_data: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Start the saga.
    pub fn start(&mut self) {
        self.status = SagaStatus::Running;
        self.updated_at = Utc::now();
    }

    /// Advance to the next step.
    pub fn advance_step(&mut self) -> Result<(), String> {
        if self.status != SagaStatus::Running {
            return Err("Saga is not running".into());
        }
        if self.current_step >= self.total_steps {
            return Err("Saga already completed".into());
        }

        self.steps[self.current_step as usize].status = SagaStepStatus::Completed;
        self.current_step += 1;
        self.updated_at = Utc::now();

        if self.current_step >= self.total_steps {
            self.status = SagaStatus::Completed;
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Fail the current step and start compensation.
    pub fn fail_step(&mut self, error: &str) {
        if self.current_step < self.total_steps {
            self.steps[self.current_step as usize].status = SagaStepStatus::Failed;
            self.steps[self.current_step as usize].error = Some(error.to_string());
        }
        self.status = SagaStatus::Compensating;
        self.updated_at = Utc::now();
    }

    /// Compensate (undo) completed steps in reverse order.
    pub fn compensate(&mut self) {
        for step in self.steps.iter_mut().rev() {
            if step.status == SagaStepStatus::Completed && step.compensation_action.is_some() {
                step.status = SagaStepStatus::Compensated;
            }
        }
        self.status = SagaStatus::Compensated;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.status, SagaStatus::Completed | SagaStatus::Compensated | SagaStatus::Failed)
    }
}

/// A single step within a saga.
#[derive(Debug, Clone)]
pub struct SagaStep {
    pub step_id: u32,
    pub name: String,
    pub service: String,
    pub action: String,
    pub compensation_action: Option<String>,
    pub status: SagaStepStatus,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl SagaStep {
    pub fn new(step_id: u32, name: String, service: String, action: String, compensation_action: Option<String>) -> Self {
        Self {
            step_id,
            name,
            service,
            action,
            compensation_action,
            status: SagaStepStatus::Pending,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_steps() -> Vec<SagaStep> {
        vec![
            SagaStep::new(0, "Create Order".into(), "order-service".into(), "create_order".into(), None),
            SagaStep::new(1, "Process Payment".into(), "orchestration-service".into(), "authorize_payment".into(), Some("void_payment".into())),
            SagaStep::new(2, "Ship Items".into(), "shipping-service".into(), "ship".into(), None),
        ]
    }

    #[test]
    fn test_new_saga_is_pending() {
        let s = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), "PaymentIntent".into(), make_steps());
        assert_eq!(s.status, SagaStatus::Pending);
        assert_eq!(s.total_steps, 3);
    }

    #[test]
    fn test_start_saga() {
        let mut s = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), "PaymentIntent".into(), make_steps());
        s.start();
        assert_eq!(s.status, SagaStatus::Running);
    }

    #[test]
    fn test_advance_step() {
        let mut s = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), "PaymentIntent".into(), make_steps());
        s.start();
        s.advance_step().unwrap();
        assert_eq!(s.current_step, 1);
        assert_eq!(s.steps[0].status, SagaStepStatus::Completed);
    }

    #[test]
    fn test_complete_saga() {
        let mut s = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), "PaymentIntent".into(), make_steps());
        s.start();
        s.advance_step().unwrap();
        s.advance_step().unwrap();
        s.advance_step().unwrap();
        assert_eq!(s.status, SagaStatus::Completed);
        assert!(s.completed_at.is_some());
        assert!(s.is_terminal());
    }

    #[test]
    fn test_fail_step() {
        let mut s = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), "PaymentIntent".into(), make_steps());
        s.start();
        s.advance_step().unwrap();
        s.fail_step("Payment declined");
        assert_eq!(s.status, SagaStatus::Compensating);
        assert_eq!(s.steps[1].error, Some("Payment declined".into()));
    }

    #[test]
    fn test_compensate() {
        let mut s = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), "PaymentIntent".into(), make_steps());
        s.start();
        s.advance_step().unwrap();
        s.fail_step("Payment declined");
        s.compensate();
        assert_eq!(s.status, SagaStatus::Compensated);
        assert!(s.is_terminal());
    }
}
