//! Saga Coordinator domain model — BC-17
//!
//! Durable state machine for multi-step, cross-aggregate workflows
//! with automatic compensation on failure.
//!
//! File structure (one concept per file per CONVENTIONS.md):
//!
//! - [`error`]       — [`SagaError`]
//! - [`status`]      — [`SagaStatus`] state machine
//! - [`saga_type`]   — [`SagaType`] known saga definitions
//! - [`step`]        — [`SagaStep`], [`StepStatus`]
//! - [`instance`]    — [`SagaInstance`] aggregate root

pub mod error;
pub mod instance;
pub mod saga_type;
pub mod status;
pub mod step;

pub use error::*;
pub use instance::*;
pub use saga_type::*;
pub use status::*;
pub use step::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_saga_status_valid_transitions() {
        assert!(SagaStatus::Created.can_transition_to(&SagaStatus::Running));
        assert!(SagaStatus::Running.can_transition_to(&SagaStatus::Completed));
        assert!(SagaStatus::Running.can_transition_to(&SagaStatus::Compensating));
        assert!(SagaStatus::Running.can_transition_to(&SagaStatus::Failed));
        assert!(SagaStatus::Compensating.can_transition_to(&SagaStatus::Compensated));
        assert!(SagaStatus::Compensating.can_transition_to(&SagaStatus::RequiresManualIntervention));
        assert!(SagaStatus::Failed.can_transition_to(&SagaStatus::RequiresManualIntervention));
    }

    #[test]
    fn test_saga_status_invalid_transitions() {
        assert!(!SagaStatus::Created.can_transition_to(&SagaStatus::Completed));
        assert!(!SagaStatus::Created.can_transition_to(&SagaStatus::Compensated));
        assert!(!SagaStatus::Completed.can_transition_to(&SagaStatus::Running));
        assert!(!SagaStatus::Compensated.can_transition_to(&SagaStatus::Running));
        assert!(!SagaStatus::RequiresManualIntervention.can_transition_to(&SagaStatus::Running));
    }

    #[test]
    fn test_saga_status_is_terminal() {
        assert!(!SagaStatus::Created.is_terminal());
        assert!(!SagaStatus::Running.is_terminal());
        assert!(SagaStatus::Completed.is_terminal());
        assert!(!SagaStatus::Compensating.is_terminal());
        assert!(SagaStatus::Compensated.is_terminal());
        assert!(!SagaStatus::Failed.is_terminal());
        assert!(SagaStatus::RequiresManualIntervention.is_terminal());
    }

    #[test]
    fn test_saga_status_display() {
        assert_eq!(format!("{}", SagaStatus::Created), "created");
        assert_eq!(format!("{}", SagaStatus::Running), "running");
        assert_eq!(format!("{}", SagaStatus::Completed), "completed");
        assert_eq!(format!("{}", SagaStatus::Compensating), "compensating");
        assert_eq!(format!("{}", SagaStatus::Compensated), "compensated");
        assert_eq!(format!("{}", SagaStatus::Failed), "failed");
        assert_eq!(
            format!("{}", SagaStatus::RequiresManualIntervention),
            "requires_manual_intervention"
        );
    }

    #[test]
    fn test_saga_type_display() {
        assert_eq!(format!("{}", SagaType::PaymentLifecycle), "payment_lifecycle");
        assert_eq!(format!("{}", SagaType::SubscriptionRenewal), "subscription_renewal");
        assert_eq!(
            format!("{}", SagaType::ReconciliationResolution),
            "reconciliation_resolution"
        );
        assert_eq!(format!("{}", SagaType::InvoicePayment), "invoice_payment");
    }

    #[test]
    fn test_saga_instance_new() {
        let steps = SagaInstance::payment_lifecycle_steps();
        let saga = SagaInstance::new(
            SagaType::PaymentLifecycle,
            Uuid::now_v7(),
            steps.clone(),
        )
        .unwrap();

        assert_eq!(saga.saga_type, SagaType::PaymentLifecycle);
        assert_eq!(saga.status, SagaStatus::Created);
        assert_eq!(saga.steps.len(), 4);
        assert_eq!(saga.compensation_attempts, 0);
        assert_eq!(saga.max_compensation_retries, 3);
        assert!(saga.deadline_at.is_some());
    }

    #[test]
    fn test_saga_instance_no_steps_rejected() {
        let result = SagaInstance::new(SagaType::PaymentLifecycle, Uuid::now_v7(), vec![]);
        assert!(result.is_err());
        assert!(matches!(result, Err(SagaError::NoStepsDefined)));
    }

    #[test]
    fn test_saga_lifecycle_happy_path() {
        let mut saga = SagaInstance::new(
            SagaType::PaymentLifecycle,
            Uuid::now_v7(),
            SagaInstance::payment_lifecycle_steps(),
        )
        .unwrap();

        // Start
        saga.start().unwrap();
        assert_eq!(saga.status, SagaStatus::Running);

        // Execute each step
        for i in 0..4 {
            let step_idx = saga.next_pending_step_index().unwrap();
            assert_eq!(step_idx, i);

            saga.start_step(step_idx).unwrap();
            saga.complete_step(step_idx, format!("output_{}", i)).unwrap();
        }

        assert_eq!(saga.status, SagaStatus::Completed);
        assert!(saga.next_pending_step().is_none());
    }

    #[test]
    fn test_saga_compensation_on_failure() {
        let mut saga = SagaInstance::new(
            SagaType::PaymentLifecycle,
            Uuid::now_v7(),
            SagaInstance::payment_lifecycle_steps(),
        )
        .unwrap();

        saga.start().unwrap();

        // Complete first two steps
        saga.start_step(0).unwrap();
        saga.complete_step(0, "ok".into()).unwrap();

        saga.start_step(1).unwrap();
        saga.complete_step(1, "ok".into()).unwrap();

        // Fail on third step
        saga.start_step(2).unwrap();
        saga.fail_step(2, "provider_error".into()).unwrap();

        assert_eq!(saga.status, SagaStatus::Compensating);

        // Compensate all succeeded steps (reverse order)
        let compensated = saga.compensate_next_step().unwrap();
        assert_eq!(compensated, Some(1)); // step 1 compensated

        let compensated = saga.compensate_next_step().unwrap();
        assert_eq!(compensated, Some(0)); // step 0 compensated

        let compensated = saga.compensate_next_step().unwrap();
        assert_eq!(compensated, None); // all done
        assert_eq!(saga.status, SagaStatus::Compensated);
    }

    #[test]
    fn test_saga_timeout() {
        let saga = SagaInstance::new(
            SagaType::PaymentLifecycle,
            Uuid::now_v7(),
            SagaInstance::payment_lifecycle_steps(),
        )
        .unwrap();

        // Not started yet → not timed out
        assert!(!saga.is_timed_out());
    }
}
