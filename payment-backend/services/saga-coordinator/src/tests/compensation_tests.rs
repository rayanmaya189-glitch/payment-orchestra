//! Compensation tests: compensation on failure, idempotent compensation.

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;

use super::{setup, create_running_saga};

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
