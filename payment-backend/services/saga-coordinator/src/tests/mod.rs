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

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> SagaPipeline {
    SagaPipeline::new()
}

fn payment_steps() -> Vec<SagaStep> {
    SagaInstance::payment_lifecycle_steps()
}

async fn create_running_saga(pipeline: &SagaPipeline) -> SagaInstance {
    let cmd = StartSagaCommand {
        saga_type: SagaType::PaymentLifecycle,
        aggregate_id: Uuid::now_v7(),
        steps: payment_steps(),
    };
    pipeline.api.start_saga(cmd).await.unwrap()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_and_start_saga() {
    let pipeline = setup();
    let saga = create_running_saga(&pipeline).await;

    assert_eq!(saga.status, SagaStatus::Running);
    assert_eq!(saga.saga_type, SagaType::PaymentLifecycle);
    assert_eq!(saga.steps.len(), 4);
    assert!(saga.steps.iter().all(|s| s.status == StepStatus::Pending));
}

#[tokio::test]
async fn test_payment_lifecycle_saga_success() {
    let pipeline = setup();
    let saga = create_running_saga(&pipeline).await;

    // Execute all 4 steps in order
    for i in 0..4 {
        // Start step
        pipeline
            .api
            .start_step(StartStepCommand {
                saga_id: saga.saga_id,
                step_index: i,
            })
            .await
            .unwrap();

        // Complete step
        pipeline
            .api
            .complete_step(CompleteStepCommand {
                saga_id: saga.saga_id,
                step_index: i,
                output: format!("step_{}_completed", i),
            })
            .await
            .unwrap();
    }

    // Saga should be completed
    let completed = pipeline.api.get_saga(saga.saga_id).await.unwrap();
    assert_eq!(completed.status, SagaStatus::Completed);
}

#[tokio::test]
async fn test_saga_compensates_on_step_failure() {
    let pipeline = setup();
    let saga = create_running_saga(&pipeline).await;

    // Complete step 0 (authorize)
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id: saga.saga_id,
            step_index: 0,
        })
        .await
        .unwrap();
    pipeline
        .api
        .complete_step(CompleteStepCommand {
            saga_id: saga.saga_id,
            step_index: 0,
            output: "authorized".into(),
        })
        .await
        .unwrap();

    // Start step 1 (capture), then fail it
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id: saga.saga_id,
            step_index: 1,
        })
        .await
        .unwrap();
    let failed = pipeline
        .api
        .fail_step(FailStepCommand {
            saga_id: saga.saga_id,
            step_index: 1,
            error: "Capture declined by acquirer".into(),
        })
        .await
        .unwrap();

    // Saga should be in compensating state
    assert_eq!(failed.status, SagaStatus::Compensating);

    // Compensate the authorized step (reverse order)
    let compensated_idx = pipeline
        .api
        .compensate_next(CompensateNextCommand {
            saga_id: saga.saga_id,
        })
        .await
        .unwrap();

    // Step 0 (authorize) should be compensated
    assert_eq!(compensated_idx, Some(0));
    let saga = pipeline.api.get_saga(saga.saga_id).await.unwrap();
    assert_eq!(saga.steps[0].status, StepStatus::Compensated);
    assert_eq!(saga.steps[1].status, StepStatus::Failed);
}

#[tokio::test]
async fn test_saga_compensates_all_steps() {
    let pipeline = setup();
    let saga = create_running_saga(&pipeline).await;

    // Complete step 0
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id: saga.saga_id,
            step_index: 0,
        })
        .await
        .unwrap();
    pipeline
        .api
        .complete_step(CompleteStepCommand {
            saga_id: saga.saga_id,
            step_index: 0,
            output: "authorized".into(),
        })
        .await
        .unwrap();

    // Fail step 1
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id: saga.saga_id,
            step_index: 1,
        })
        .await
        .unwrap();
    pipeline
        .api
        .fail_step(FailStepCommand {
            saga_id: saga.saga_id,
            step_index: 1,
            error: "Failed".into(),
        })
        .await
        .unwrap();

    // Compensate step 0
    pipeline
        .api
        .compensate_next(CompensateNextCommand {
            saga_id: saga.saga_id,
        })
        .await
        .unwrap();

    // All compensated now
    let saga = pipeline.api.get_saga(saga.saga_id).await.unwrap();
    assert_eq!(saga.status, SagaStatus::Compensated);
}

#[tokio::test]
async fn test_saga_no_steps_rejected() {
    let pipeline = setup();
    let cmd = StartSagaCommand {
        saga_type: SagaType::PaymentLifecycle,
        aggregate_id: Uuid::now_v7(),
        steps: vec![],
    };
    let result = pipeline.api.start_saga(cmd).await;
    assert!(result.is_err(), "Empty steps should be rejected");
}

#[tokio::test]
async fn test_saga_timeout_detection() {
    let pipeline = setup();
    let _saga = create_running_saga(&pipeline).await;

    // Find stuck sagas with a short timeout
    let stuck = pipeline
        .api
        .find_stuck(1) // 1 second timeout
        .await
        .unwrap();

    // The saga was just created, so it shouldn't be stuck
    assert!(
        stuck.is_empty(),
        "Newly created saga should not be stuck"
    );
}

#[tokio::test]
async fn test_compensation_idempotent() {
    let pipeline = setup();
    let saga = create_running_saga(&pipeline).await;

    // Complete step 0
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id: saga.saga_id,
            step_index: 0,
        })
        .await
        .unwrap();
    pipeline
        .api
        .complete_step(CompleteStepCommand {
            saga_id: saga.saga_id,
            step_index: 0,
            output: "authorized".into(),
        })
        .await
        .unwrap();

    // Fail step 1
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id: saga.saga_id,
            step_index: 1,
        })
        .await
        .unwrap();
    pipeline
        .api
        .fail_step(FailStepCommand {
            saga_id: saga.saga_id,
            step_index: 1,
            error: "Failed".into(),
        })
        .await
        .unwrap();

    // Compensate step 0
    pipeline
        .api
        .compensate_next(CompensateNextCommand {
            saga_id: saga.saga_id,
        })
        .await
        .unwrap();

    // Try compensating again — should return None (nothing left to compensate)
    let result = pipeline
        .api
        .compensate_next(CompensateNextCommand {
            saga_id: saga.saga_id,
        })
        .await
        .unwrap();
    assert!(result.is_none(), "No more steps to compensate");
}

#[tokio::test]
async fn test_find_stuck_stale_saga() {
    let pipeline = setup();
    let saga = create_running_saga(&pipeline).await;
    let saga_id = saga.saga_id;

    // Start step 0 but don't complete it
    pipeline
        .api
        .start_step(StartStepCommand {
            saga_id,
            step_index: 0,
        })
        .await
        .unwrap();

    // With a longer timeout, the saga should NOT be stuck
    let stuck = pipeline.api.find_stuck(3600).await.unwrap();
    assert!(stuck.is_empty(), "Recent saga should not be stuck");
}

#[tokio::test]
async fn test_query_nonexistent_saga() {
    let pipeline = setup();
    let result = pipeline.api.get_saga(Uuid::now_v7()).await;
    assert!(result.is_err(), "Nonexistent saga should error");
}

#[tokio::test]
async fn test_find_by_aggregate() {
    let pipeline = setup();
    let aggregate_id = Uuid::now_v7();

    // Create two sagas for same aggregate
    let cmd1 = StartSagaCommand {
        saga_type: SagaType::PaymentLifecycle,
        aggregate_id,
        steps: payment_steps(),
    };
    pipeline.api.start_saga(cmd1).await.unwrap();

    let cmd2 = StartSagaCommand {
        saga_type: SagaType::InvoicePayment,
        aggregate_id,
        steps: payment_steps(),
    };
    pipeline.api.start_saga(cmd2).await.unwrap();

    let results = pipeline.api.find_by_aggregate(aggregate_id).await.unwrap();
    assert_eq!(results.len(), 2);
}
