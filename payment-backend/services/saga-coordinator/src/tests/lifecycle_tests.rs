//! Lifecycle tests: create, start, full lifecycle, queries.

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;

use super::{setup, payment_steps, create_running_saga};

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
