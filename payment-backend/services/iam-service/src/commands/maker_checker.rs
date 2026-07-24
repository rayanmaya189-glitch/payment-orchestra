//! Maker-checker (submit/review change) command handlers.

use tracing::info;

use crate::domain::{PendingChange, IamError};
use crate::repository::IamRepository;
use super::types::*;
use super::IamCommandHandler;

impl<R: IamRepository + Send + Sync> IamCommandHandler<R> {
    pub(crate) async fn submit_change_impl(&self, cmd: SubmitChange) -> Result<SubmitChangeResult, IamError> {
        let change = PendingChange::new(
            cmd.change_type,
            cmd.maker_id,
            cmd.payload,
            cmd.maker_note,
        );

        self.repository.save_change(&change).await?;

        info!(change_id = %change.change_id, type = %change.change_type, "Pending change submitted");

        Ok(SubmitChangeResult { change })
    }

    pub(crate) async fn review_change_impl(&self, cmd: ReviewChange) -> Result<ReviewChangeResult, IamError> {
        let mut change = self.repository.load_change(cmd.change_id).await?
            .ok_or_else(|| IamError::InvalidRequest("Change not found".into()))?;

        if cmd.approved {
            change.approve(cmd.checker_id, cmd.checker_note)?;
        } else {
            change.reject(cmd.checker_id, cmd.checker_note)?;
        }

        self.repository.save_change(&change).await?;

        info!(change_id = %change.change_id, status = %change.status.as_str(), "Change reviewed");

        Ok(ReviewChangeResult { change })
    }
}
