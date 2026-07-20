use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{SagaStatus, SagaStepStatus};

#[derive(Debug, Clone)]
pub struct SagaInstance {
    pub saga_id: Uuid, pub saga_type: String, pub status: SagaStatus,
    pub current_step: u32, pub total_steps: u32,
    pub steps: Vec<SagaStep>, pub payload: serde_json::Value,
    pub compensation_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct SagaStep {
    pub step_number: u32, pub name: String, pub service: String,
    pub action: String, pub compensation_action: Option<String>,
    pub status: SagaStepStatus, pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>, pub completed_at: Option<DateTime<Utc>>,
}

impl SagaInstance {
    pub fn new(saga_type: String, steps: Vec<SagaStep>, payload: serde_json::Value) -> Self {
        let total = steps.len() as u32;
        let now = Utc::now();
        Self { saga_id: Uuid::now_v7(), saga_type, status: SagaStatus::Running,
            current_step: 1, total_steps: total, steps, payload,
            compensation_data: None, created_at: now, updated_at: now, completed_at: None }
    }

    pub fn advance_step(&mut self) -> Option<&SagaStep> {
        if self.current_step <= self.total_steps {
            let step = &mut self.steps[(self.current_step - 1) as usize];
            step.status = SagaStepStatus::Running;
            step.started_at = Some(Utc::now());
            self.updated_at = Utc::now();
            Some(&self.steps[(self.current_step - 1) as usize])
        } else {
            None
        }
    }

    pub fn complete_step(&mut self) {
        let idx = (self.current_step - 1) as usize;
        self.steps[idx].status = SagaStepStatus::Completed;
        self.steps[idx].completed_at = Some(Utc::now());
        self.current_step += 1;
        self.updated_at = Utc::now();
        if self.current_step > self.total_steps {
            self.status = SagaStatus::Completed;
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn fail_step(&mut self, error: &str) {
        let idx = (self.current_step - 1) as usize;
        self.steps[idx].status = SagaStepStatus::Failed;
        self.steps[idx].error = Some(error.to_string());
        self.status = SagaStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn compensate(&mut self) {
        self.status = SagaStatus::Compensating;
        for step in self.steps.iter_mut().rev() {
            if step.status == SagaStepStatus::Completed && step.compensation_action.is_some() {
                step.status = SagaStepStatus::Compensated;
            }
        }
        self.status = SagaStatus::Compensated;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_steps() -> Vec<SagaStep> {
        vec![
            SagaStep { step_number: 1, name: "charge".into(), service: "orchestration".into(), action: "authorize".into(), compensation_action: Some("void".into()), status: SagaStepStatus::Pending, error: None, started_at: None, completed_at: None },
            SagaStep { step_number: 2, name: "notify".into(), service: "notification".into(), action: "send".into(), compensation_action: None, status: SagaStepStatus::Pending, error: None, started_at: None, completed_at: None },
        ]
    }

    #[test]
    fn test_new_saga() {
        let s = SagaInstance::new("payment".into(), make_steps(), serde_json::json!({}));
        assert_eq!(s.status, SagaStatus::Running);
        assert_eq!(s.total_steps, 2);
    }

    #[test]
    fn test_advance_and_complete() {
        let mut s = SagaInstance::new("payment".into(), make_steps(), serde_json::json!({}));
        s.advance_step();
        s.complete_step();
        assert_eq!(s.current_step, 2);
        assert_eq!(s.status, SagaStatus::Running);
        s.advance_step();
        s.complete_step();
        assert_eq!(s.status, SagaStatus::Completed);
    }

    #[test]
    fn test_fail_step() {
        let mut s = SagaInstance::new("payment".into(), make_steps(), serde_json::json!({}));
        s.advance_step();
        s.fail_step("timeout");
        assert_eq!(s.status, SagaStatus::Failed);
    }

    #[test]
    fn test_compensate() {
        let mut s = SagaInstance::new("payment".into(), make_steps(), serde_json::json!({}));
        s.advance_step();
        s.complete_step();
        s.advance_step();
        s.fail_step("error");
        s.compensate();
        assert_eq!(s.status, SagaStatus::Compensated);
    }
}
