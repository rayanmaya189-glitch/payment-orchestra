//! Saga Coordinator TDD tests — BC-17
//!
//! Spec tests:
//! - test_payment_lifecycle_saga_success: full lifecycle completed
//! - test_saga_compensates_on_step_failure: failure triggers reverse-order compensation
//! - test_saga_compensates_all_steps: all succeeded steps compensated
//! - test_saga_no_steps_rejected: empty steps rejected
//! - test_saga_timeout_detection: stuck saga detection
//! - test_compensation_idempotent: compensate already compensated steps
//! - test_complete_saga_already_completed: idempotent check
//! - test_get_saga: query by ID
//! - test_find_stuck: timeout query

mod compensation_tests;
mod lifecycle_tests;
mod timeout_tests;
mod pg_repository_tests;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

pub(crate) fn setup() -> SagaPipeline {
    SagaPipeline::new()
}

pub(crate) fn payment_steps() -> Vec<SagaStep> {
    SagaInstance::payment_lifecycle_steps()
}

pub(crate) async fn create_running_saga(pipeline: &SagaPipeline) -> SagaInstance {
    let cmd = StartSagaCommand {
        saga_type: SagaType::PaymentLifecycle,
        aggregate_id: Uuid::now_v7(),
        steps: payment_steps(),
    };
    pipeline.api.start_saga(cmd).await.unwrap()
}
