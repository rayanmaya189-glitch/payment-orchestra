use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::{SagaInstance, SagaStep};
use crate::domain::rules::SagaRepository;
use crate::domain::value_objects::SagaStepStatus;
use platform_error::PlatformError;

pub struct SagaServiceImpl { repo: Box<dyn SagaRepository>, db: DatabaseConnection }
impl SagaServiceImpl { pub fn new(repo: Box<dyn SagaRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait SagaService: Send + Sync {
    async fn start(&self, cmd: StartSagaCommand) -> Result<Uuid, PlatformError>;
    async fn advance(&self, cmd: AdvanceSagaCommand) -> Result<(), PlatformError>;
    async fn fail(&self, cmd: FailSagaCommand) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<SagaInstance, PlatformError>;
}

#[async_trait]
impl SagaService for SagaServiceImpl {
    async fn start(&self, cmd: StartSagaCommand) -> Result<Uuid, PlatformError> {
        let steps: Vec<SagaStep> = cmd.steps.into_iter().enumerate().map(|(i, s)| {
            SagaStep { step_number: (i + 1) as u32, name: s.name, service: s.service, action: s.action,
                compensation_action: s.compensation_action, status: SagaStepStatus::Pending,
                error: None, started_at: None, completed_at: None }
        }).collect();
        let saga = SagaInstance::new(cmd.saga_type, steps, cmd.payload);
        self.repo.save(&saga).await?;
        Ok(saga.saga_id)
    }

    async fn advance(&self, cmd: AdvanceSagaCommand) -> Result<(), PlatformError> {
        let mut saga = self.repo.find_by_id(cmd.saga_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "saga".into(), id: cmd.saga_id })?;
        saga.advance_step();
        saga.complete_step();
        self.repo.save(&saga).await
    }

    async fn fail(&self, cmd: FailSagaCommand) -> Result<(), PlatformError> {
        let mut saga = self.repo.find_by_id(cmd.saga_id).await?
            .ok_or_else(|| PlatformError::NotFound { resource: "saga".into(), id: cmd.saga_id })?;
        saga.fail_step(&cmd.error);
        saga.compensate();
        self.repo.save(&saga).await
    }

    async fn get(&self, id: Uuid) -> Result<SagaInstance, PlatformError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| PlatformError::NotFound { resource: "saga".into(), id })
    }
}
