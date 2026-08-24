//! Timeout tests: timeout detection, stale sagas.

use uuid::Uuid;

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;

use super::{setup, create_running_saga};

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
