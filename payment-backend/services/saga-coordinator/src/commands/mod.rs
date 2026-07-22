//! Saga Coordinator command handlers — BC-17

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

/// Start a new saga execution.
pub struct StartSagaCommand {
    pub saga_type: SagaType,
    pub aggregate_id: Uuid,
    pub steps: Vec<SagaStep>,
}

/// Begin execution of a saga (Created → Running).
pub struct BeginSagaCommand {
    pub saga_id: Uuid,
}

/// Mark a step as started.
pub struct StartStepCommand {
    pub saga_id: Uuid,
    pub step_index: usize,
}

/// Mark a step as completed.
pub struct CompleteStepCommand {
    pub saga_id: Uuid,
    pub step_index: usize,
    pub output: String,
}

/// Mark a step as failed (triggers compensation).
pub struct FailStepCommand {
    pub saga_id: Uuid,
    pub step_index: usize,
    pub error: String,
}

/// Compensate the next succeeded step (reverse order).
pub struct CompensateNextCommand {
    pub saga_id: Uuid,
}

/// Mark saga as completed.
pub struct CompleteSagaCommand {
    pub saga_id: Uuid,
}

/// Mark saga as failed (non-compensated).
pub struct FailSagaCommand {
    pub saga_id: Uuid,
    pub error: String,
}

// ---------------------------------------------------------------------------
// Command handler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn start_saga(&self, cmd: StartSagaCommand) -> Result<SagaInstance, SagaError>;
    async fn begin_saga(&self, cmd: BeginSagaCommand) -> Result<SagaInstance, SagaError>;
    async fn start_step(&self, cmd: StartStepCommand) -> Result<SagaInstance, SagaError>;
    async fn complete_step(&self, cmd: CompleteStepCommand) -> Result<SagaInstance, SagaError>;
    async fn fail_step(&self, cmd: FailStepCommand) -> Result<SagaInstance, SagaError>;
    async fn compensate_next(&self, cmd: CompensateNextCommand) -> Result<Option<usize>, SagaError>;
    async fn complete_saga(&self, cmd: CompleteSagaCommand) -> Result<SagaInstance, SagaError>;
    async fn fail_saga(&self, cmd: FailSagaCommand) -> Result<SagaInstance, SagaError>;
}

// ---------------------------------------------------------------------------
// Handler implementation
// ---------------------------------------------------------------------------

pub struct SagaCommandHandler<R: SagaRepository> {
    repo: R,
}

impl<R: SagaRepository> SagaCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: SagaRepository + Send + Sync> CommandHandler for SagaCommandHandler<R> {
    async fn start_saga(&self, cmd: StartSagaCommand) -> Result<SagaInstance, SagaError> {
        let mut saga = SagaInstance::new(cmd.saga_type, cmd.aggregate_id, cmd.steps)?;
        saga.start()?;
        self.repo.save(&saga).await?;
        Ok(saga)
    }

    async fn begin_saga(&self, cmd: BeginSagaCommand) -> Result<SagaInstance, SagaError> {
        let mut saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        saga.start()?;
        self.repo.save(&saga).await?;
        Ok(saga)
    }

    async fn start_step(&self, cmd: StartStepCommand) -> Result<SagaInstance, SagaError> {
        let mut saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        saga.start_step(cmd.step_index)?;
        self.repo.save(&saga).await?;
        Ok(saga)
    }

    async fn complete_step(&self, cmd: CompleteStepCommand) -> Result<SagaInstance, SagaError> {
        let mut saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        saga.complete_step(cmd.step_index, cmd.output)?;
        self.repo.save(&saga).await?;
        Ok(saga)
    }

    async fn fail_step(&self, cmd: FailStepCommand) -> Result<SagaInstance, SagaError> {
        let mut saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        saga.fail_step(cmd.step_index, cmd.error)?;
        self.repo.save(&saga).await?;
        Ok(saga)
    }

    async fn compensate_next(
        &self,
        cmd: CompensateNextCommand,
    ) -> Result<Option<usize>, SagaError> {
        let mut saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        let result = saga.compensate_next_step()?;
        self.repo.save(&saga).await?;
        Ok(result)
    }

    async fn complete_saga(&self, cmd: CompleteSagaCommand) -> Result<SagaInstance, SagaError> {
        let saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        if saga.status != SagaStatus::Completed && !saga.status.is_terminal() {
            return Err(SagaError::InvalidTransition);
        }

        Ok(saga)
    }

    async fn fail_saga(&self, cmd: FailSagaCommand) -> Result<SagaInstance, SagaError> {
        let mut saga = self
            .repo
            .load(cmd.saga_id)
            .await?
            .ok_or(SagaError::NotFound(cmd.saga_id))?;

        saga.mark_failed(cmd.error)?;
        self.repo.save(&saga).await?;
        Ok(saga)
    }
}
