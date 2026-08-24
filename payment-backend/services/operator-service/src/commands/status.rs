//! Update operator status command handler.

use chrono::Utc;
use tracing::info;

use crate::domain::{OperatorError, OperatorStatus};
use crate::events::{OperatorEvent, OperatorSuspended, OperatorReactivated};
use crate::repository::OperatorRepository;
use super::types::*;
use super::OperatorCommandHandler;

impl<R: OperatorRepository + Send + Sync> OperatorCommandHandler<R> {
    pub(crate) async fn update_status_impl(&self, cmd: UpdateOperatorStatus) -> Result<UpdateOperatorStatusResult, OperatorError> {
        let mut operator = self.repository.load(cmd.operator_id).await?
            .ok_or(OperatorError::NotFound(cmd.operator_id))?;

        let previous_status = operator.status.clone();
        operator.update_status(cmd.new_status)?;
        self.repository.save(&mut operator).await?;

        // Publish appropriate event
        match operator.status {
            OperatorStatus::Suspended => {
                self.publish_event(OperatorEvent::Suspended(OperatorSuspended {
                    operator_id: operator.id,
                    reason: cmd.reason.clone(),
                    occurred_at: Utc::now(),
                }));
            }
            OperatorStatus::ActiveVerified if previous_status == OperatorStatus::Suspended => {
                self.publish_event(OperatorEvent::Reactivated(OperatorReactivated {
                    operator_id: operator.id,
                    reason: cmd.reason,
                    occurred_at: Utc::now(),
                }));
            }
            _ => {}
        }

        info!(
            operator_id = %operator.id,
            status = %operator.status.as_str(),
            "Operator status updated"
        );

        Ok(UpdateOperatorStatusResult { operator })
    }
}
