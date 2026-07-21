use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::{SagaInstance, SagaStep, PaymentSagaSteps};
use crate::domain::rules::SagaRepository;
use crate::domain::value_objects::{SagaStatus, SagaStepStatus};
use crate::infrastructure::messaging::EventPublisher;
use platform_error::PlatformError;
use shared_types::events::EventEnvelope;

pub struct SagaServiceImpl {
    repo: Box<dyn SagaRepository>,
    db: DatabaseConnection,
    publisher: Option<EventPublisher>,
}

impl SagaServiceImpl {
    pub fn new(repo: Box<dyn SagaRepository>, db: DatabaseConnection) -> Self {
        Self {
            repo,
            db,
            publisher: None,
        }
    }

    pub fn with_publisher(mut self, publisher: EventPublisher) -> Self {
        self.publisher = Some(publisher);
        self
    }

    async fn publish_event(
        &self,
        saga: &SagaInstance,
        event_type: &str,
        extra: serde_json::Value,
    ) {
        if let Some(ref publisher) = self.publisher {
            let mut payload = serde_json::json!({
                "saga_id": saga.saga_id,
                "saga_type": saga.saga_type,
                "status": saga.status.as_str(),
                "current_step": saga.current_step,
                "total_steps": saga.total_steps,
            });
            if let Some(obj) = payload.as_object_mut() {
                if let Some(extra_obj) = extra.as_object() {
                    for (k, v) in extra_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }

            let event = EventEnvelope::new(
                "SagaInstance",
                saga.saga_id,
                event_type,
                "saga_coordinator",
                saga.saga_id,
                payload,
            );

            if let Err(e) = publisher.publish(&event).await {
                tracing::warn!("Failed to publish event {event_type}: {e}");
            }
        }
    }

    /// Create a payment lifecycle saga with the predefined authorize->capture->settle steps.
    async fn start_payment_lifecycle(
        &self,
        payload: serde_json::Value,
        started_by: Option<Uuid>,
    ) -> Result<SagaInstance, PlatformError> {
        let steps = PaymentSagaSteps::payment_lifecycle();
        let mut saga = SagaInstance::new("payment_lifecycle".into(), steps, payload);
        if let Some(actor) = started_by {
            saga.set_compensation_data(serde_json::json!({"started_by": actor}));
        }
        self.repo.save(&saga).await?;
        self.publish_event(&saga, "SagaStarted", serde_json::json!({}))
            .await;
        Ok(saga)
    }

    /// Validate that a command's ABAC context allows the action.
    fn authorize(
        &self,
        role: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), PlatformError> {
        let ctx = platform_middleware::AbacContext {
            principal_id: Uuid::nil(), // validated at middleware layer
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        platform_middleware::evaluate_policy(&ctx)
    }
}

#[async_trait]
pub trait SagaService: Send + Sync {
    async fn start(&self, cmd: StartSagaCommand) -> Result<SagaInstance, PlatformError>;
    async fn start_payment_lifecycle(
        &self,
        payload: serde_json::Value,
        started_by: Option<Uuid>,
    ) -> Result<SagaInstance, PlatformError>;
    async fn advance(&self, cmd: AdvanceSagaCommand) -> Result<SagaStep, PlatformError>;
    async fn complete(&self, cmd: AdvanceSagaCommand) -> Result<(), PlatformError>;
    async fn fail(&self, cmd: FailSagaCommand) -> Result<(), PlatformError>;
    async fn compensate(&self, cmd: CompensateSagaCommand) -> Result<Vec<SagaStep>, PlatformError>;
    async fn mark_step_compensated(
        &self,
        cmd: MarkStepCompensatedCommand,
    ) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<SagaInstance, PlatformError>;
    async fn list(&self, query: ListSagasQuery) -> Result<Vec<SagaInstance>, PlatformError>;
}

#[async_trait]
impl SagaService for SagaServiceImpl {
    async fn start(&self, cmd: StartSagaCommand) -> Result<SagaInstance, PlatformError> {
        let steps = if cmd.saga_type == "payment_lifecycle" && cmd.steps.is_empty() {
            PaymentSagaSteps::payment_lifecycle()
        } else if cmd.steps.is_empty() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField("steps".into()),
            ));
        } else {
            PaymentSagaSteps::from_defs(cmd.steps)
        };

        let mut saga = SagaInstance::new(cmd.saga_type, steps, cmd.payload);
        if let Some(actor) = cmd.started_by {
            saga.set_compensation_data(serde_json::json!({"started_by": actor}));
        }

        self.repo.save(&saga).await?;
        self.publish_event(&saga, "SagaStarted", serde_json::json!({}))
            .await;
        Ok(saga)
    }

    async fn start_payment_lifecycle(
        &self,
        payload: serde_json::Value,
        started_by: Option<Uuid>,
    ) -> Result<SagaInstance, PlatformError> {
        self.start_payment_lifecycle(payload, started_by).await
    }

    async fn advance(&self, cmd: AdvanceSagaCommand) -> Result<SagaStep, PlatformError> {
        let mut saga = self
            .repo
            .find_by_id(cmd.saga_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "saga".into(),
                id: cmd.saga_id,
            })?;

        let step = saga
            .advance_step()
            .map_err(|e| PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: saga.status.as_str().to_string(),
                    command: format!("advance: {e}"),
                },
            ))?;

        self.repo.save(&saga).await?;
        self.publish_event(
            &saga,
            "SagaStepStarted",
            serde_json::json!({
                "step_number": step.step_number,
                "step_name": step.name,
                "action": step.action,
            }),
        )
        .await;

        Ok(step)
    }

    async fn complete(&self, cmd: AdvanceSagaCommand) -> Result<(), PlatformError> {
        let mut saga = self
            .repo
            .find_by_id(cmd.saga_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "saga".into(),
                id: cmd.saga_id,
            })?;

        saga.complete_step().map_err(|e| PlatformError::Validation(
            platform_error::ValidationError::InvalidStateTransition {
                from: saga.status.as_str().to_string(),
                command: format!("complete: {e}"),
            },
        ))?;

        self.repo.save(&saga).await?;
        self.publish_event(
            &saga,
            "SagaStepCompleted",
            serde_json::json!({
                "step_number": saga.current_step.saturating_sub(1),
                "status": saga.status.as_str(),
            }),
        )
        .await;

        Ok(())
    }

    async fn fail(&self, cmd: FailSagaCommand) -> Result<(), PlatformError> {
        let mut saga = self
            .repo
            .find_by_id(cmd.saga_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "saga".into(),
                id: cmd.saga_id,
            })?;

        saga.fail_step(&cmd.error).map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: saga.status.as_str().to_string(),
                command: format!("fail: {e}"),
            })
        })?;

        self.repo.save(&saga).await?;
        self.publish_event(
            &saga,
            "SagaStepFailed",
            serde_json::json!({
                "step_number": saga.current_step,
                "error": cmd.error,
                "error_code": cmd.error_code,
            }),
        )
        .await;

        Ok(())
    }

    async fn compensate(&self, cmd: CompensateSagaCommand) -> Result<Vec<SagaStep>, PlatformError> {
        let mut saga = self
            .repo
            .find_by_id(cmd.saga_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "saga".into(),
                id: cmd.saga_id,
            })?;

        let steps_to_compensate = saga.begin_compensation().map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: saga.status.as_str().to_string(),
                command: format!("compensate: {e}"),
            })
        })?;

        if let Some(reason) = cmd.reason {
            saga.set_compensation_data(serde_json::json!({"reason": reason}));
        }

        self.repo.save(&saga).await?;
        self.publish_event(
            &saga,
            "SagaCompensationStarted",
            serde_json::json!({
                "steps_to_compensate": steps_to_compensate.len(),
            }),
        )
        .await;

        Ok(steps_to_compensate)
    }

    async fn mark_step_compensated(
        &self,
        cmd: MarkStepCompensatedCommand,
    ) -> Result<(), PlatformError> {
        let mut saga = self
            .repo
            .find_by_id(cmd.saga_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "saga".into(),
                id: cmd.saga_id,
            })?;

        saga.mark_step_compensated(cmd.step_number).map_err(|e| {
            PlatformError::Validation(platform_error::ValidationError::InvalidStateTransition {
                from: saga.status.as_str().to_string(),
                command: format!("mark_compensated: {e}"),
            })
        })?;

        self.repo.save(&saga).await?;
        self.publish_event(
            &saga,
            "SagaStepCompensated",
            serde_json::json!({"step_number": cmd.step_number}),
        )
        .await;

        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<SagaInstance, PlatformError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "saga".into(),
                id,
            })
    }

    async fn list(&self, query: ListSagasQuery) -> Result<Vec<SagaInstance>, PlatformError> {
        if let (Some(saga_type), Some(status)) = (&query.saga_type, &query.status) {
            let limit = query.limit.unwrap_or(50);
            self.repo
                .find_by_type_and_status(saga_type, status, limit)
                .await
        } else {
            // For now, require both filters or return empty
            // A production impl would have a more flexible query
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapters::noop_saga_repository::NoopSagaRepository;
    use crate::domain::aggregates::SagaStepDef;

    fn make_service() -> SagaServiceImpl {
        let repo = NoopSagaRepository::new();
        let db = sea_orm::DatabaseConnection::default();
        SagaServiceImpl::new(Box::new(repo), db)
    }

    #[tokio::test]
    async fn test_start_saga() {
        let svc = make_service();
        let cmd = StartSagaCommand {
            saga_type: "payment".into(),
            steps: vec![
                SagaStepDef {
                    name: "authorize".into(),
                    service: "orchestration".into(),
                    action: "authorize".into(),
                    compensation_action: Some("void".into()),
                },
            ],
            payload: serde_json::json!({"amount": 1000}),
            started_by: None,
        };
        let saga = svc.start(cmd).await.unwrap();
        assert_eq!(saga.status, SagaStatus::Running);
        assert_eq!(saga.total_steps, 1);
    }

    #[tokio::test]
    async fn test_start_payment_lifecycle() {
        let svc = make_service();
        let saga = svc
            .start_payment_lifecycle(
                serde_json::json!({"amount": 5000}),
                Some(Uuid::now_v7()),
            )
            .await
            .unwrap();
        assert_eq!(saga.saga_type, "payment_lifecycle");
        assert_eq!(saga.total_steps, 3);
    }

    #[tokio::test]
    async fn test_start_without_steps_or_type_fails() {
        let svc = make_service();
        let cmd = StartSagaCommand {
            saga_type: "unknown".into(),
            steps: vec![],
            payload: serde_json::json!({}),
            started_by: None,
        };
        let err = svc.start(cmd).await.unwrap_err();
        assert!(matches!(err, PlatformError::Validation(_)));
    }

    #[tokio::test]
    async fn test_advance_and_fail() {
        let svc = make_service();
        let saga = svc
            .start(StartSagaCommand {
                saga_type: "payment".into(),
                steps: vec![
                    SagaStepDef {
                        name: "step1".into(),
                        service: "svc".into(),
                        action: "act".into(),
                        compensation_action: Some("comp".into()),
                    },
                ],
                payload: serde_json::json!({}),
                started_by: None,
            })
            .await
            .unwrap();

        let step = svc
            .advance(AdvanceSagaCommand {
                saga_id: saga.saga_id,
                step_result: None,
            })
            .await
            .unwrap();
        assert_eq!(step.name, "step1");

        svc.fail(FailSagaCommand {
            saga_id: saga.saga_id,
            error: "timeout".into(),
            error_code: Some("TIMEOUT".into()),
        })
        .await
        .unwrap();

        let fetched = svc.get(saga.saga_id).await.unwrap();
        assert_eq!(fetched.status, SagaStatus::Failed);
    }

    #[tokio::test]
    async fn test_compensate_flow() {
        let svc = make_service();
        let saga = svc
            .start(StartSagaCommand {
                saga_type: "payment".into(),
                steps: vec![
                    SagaStepDef {
                        name: "auth".into(),
                        service: "svc".into(),
                        action: "authorize".into(),
                        compensation_action: Some("void".into()),
                    },
                    SagaStepDef {
                        name: "capture".into(),
                        service: "svc".into(),
                        action: "capture".into(),
                        compensation_action: Some("refund".into()),
                    },
                ],
                payload: serde_json::json!({}),
                started_by: None,
            })
            .await
            .unwrap();

        // Complete step 1
        svc.advance(AdvanceSagaCommand {
            saga_id: saga.saga_id,
            step_result: None,
        })
        .await
        .unwrap();
        svc.complete(AdvanceSagaCommand {
            saga_id: saga.saga_id,
            step_result: None,
        })
        .await
        .unwrap();

        // Start step 2
        svc.advance(AdvanceSagaCommand {
            saga_id: saga.saga_id,
            step_result: None,
        })
        .await
        .unwrap();

        svc.fail(FailSagaCommand {
            saga_id: saga.saga_id,
            error: "error".into(),
            error_code: None,
        })
        .await
        .unwrap();

        // Compensate
        let steps = svc
            .compensate(CompensateSagaCommand {
                saga_id: saga.saga_id,
                reason: Some("step 2 failed".into()),
            })
            .await
            .unwrap();
        assert_eq!(steps.len(), 1); // only step 1 was completed
        assert_eq!(steps[0].name, "auth");

        // Mark compensated
        svc.mark_step_compensated(MarkStepCompensatedCommand {
            saga_id: saga.saga_id,
            step_number: 1,
        })
        .await
        .unwrap();

        let fetched = svc.get(saga.saga_id).await.unwrap();
        assert_eq!(fetched.status, SagaStatus::Compensated);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let svc = make_service();
        let err = svc.get(Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, PlatformError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_advance_not_found() {
        let svc = make_service();
        let err = svc
            .advance(AdvanceSagaCommand {
                saga_id: Uuid::now_v7(),
                step_result: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, PlatformError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_fail_on_completed_saga() {
        let svc = make_service();
        let saga = svc
            .start(StartSagaCommand {
                saga_type: "test".into(),
                steps: vec![SagaStepDef {
                    name: "s1".into(),
                    service: "svc".into(),
                    action: "a".into(),
                    compensation_action: None,
                }],
                payload: serde_json::json!({}),
                started_by: None,
            })
            .await
            .unwrap();

        svc.advance(AdvanceSagaCommand {
            saga_id: saga.saga_id,
            step_result: None,
        })
        .await
        .unwrap();
        svc.complete(AdvanceSagaCommand {
            saga_id: saga.saga_id,
            step_result: None,
        })
        .await
        .unwrap();
        // Now saga is completed
        let err = svc
            .fail(FailSagaCommand {
                saga_id: saga.saga_id,
                error: "too late".into(),
                error_code: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition { .. }
            )
        ));
    }
}
