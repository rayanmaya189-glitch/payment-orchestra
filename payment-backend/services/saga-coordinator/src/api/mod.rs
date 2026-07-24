//! Saga Coordinator API surface — BC-17
//!
//! Includes both the local API struct and the gRPC service implementation.

pub mod grpc;

use crate::commands::*;
use crate::domain::{SagaError, SagaInstance};
use crate::queries::*;
use uuid::Uuid;

pub struct SagaApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl SagaApi {
    pub fn new(
        command_handler: Box<dyn CommandHandler>,
        query_handler: Box<dyn QueryHandler>,
    ) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }

    pub async fn start_saga(&self, cmd: StartSagaCommand) -> Result<SagaInstance, SagaError> {
        self.command_handler.start_saga(cmd).await
    }

    pub async fn begin_saga(&self, cmd: BeginSagaCommand) -> Result<SagaInstance, SagaError> {
        self.command_handler.begin_saga(cmd).await
    }

    pub async fn start_step(&self, cmd: StartStepCommand) -> Result<SagaInstance, SagaError> {
        self.command_handler.start_step(cmd).await
    }

    pub async fn complete_step(
        &self,
        cmd: CompleteStepCommand,
    ) -> Result<SagaInstance, SagaError> {
        self.command_handler.complete_step(cmd).await
    }

    pub async fn fail_step(&self, cmd: FailStepCommand) -> Result<SagaInstance, SagaError> {
        self.command_handler.fail_step(cmd).await
    }

    pub async fn compensate_next(
        &self,
        cmd: CompensateNextCommand,
    ) -> Result<Option<usize>, SagaError> {
        self.command_handler.compensate_next(cmd).await
    }

    pub async fn complete_saga(
        &self,
        cmd: CompleteSagaCommand,
    ) -> Result<SagaInstance, SagaError> {
        self.command_handler.complete_saga(cmd).await
    }

    pub async fn fail_saga(&self, cmd: FailSagaCommand) -> Result<SagaInstance, SagaError> {
        self.command_handler.fail_saga(cmd).await
    }

    pub async fn get_saga(&self, id: Uuid) -> Result<SagaInstance, SagaError> {
        self.query_handler.get_saga(id).await
    }

    pub async fn find_by_aggregate(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<SagaInstance>, SagaError> {
        self.query_handler.find_by_aggregate(aggregate_id).await
    }

    pub async fn find_stuck(
        &self,
        timeout_seconds: i64,
    ) -> Result<Vec<SagaInstance>, SagaError> {
        self.query_handler.find_stuck(timeout_seconds).await
    }
}
