//! Command handler trait and implementation for dispute-service.

use async_trait::async_trait;

use crate::domain::*;
use crate::repository::*;
use super::types::*;

// ─── Command Handler Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn record_chargeback(&self, cmd: RecordChargebackCommand) -> Result<ChargebackCase, DisputeError>;
    async fn submit_representment(&self, cmd: SubmitRepresentmentCommand) -> Result<ChargebackCase, DisputeError>;
    async fn resolve_chargeback(&self, cmd: ResolveChargebackCommand) -> Result<ChargebackCase, DisputeError>;
}

// ─── Handler Implementation ──────────────────────────────────────────────────

pub struct DisputeCommandHandler<R: DisputeRepository> {
    repo: R,
}

impl<R: DisputeRepository> DisputeCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: DisputeRepository + Send + Sync> CommandHandler for DisputeCommandHandler<R> {
    async fn record_chargeback(&self, cmd: RecordChargebackCommand) -> Result<ChargebackCase, DisputeError> {
        if !cmd.is_captured {
            return Err(DisputeError::PaymentIntentNotCaptured);
        }

        let case = ChargebackCase::new(
            cmd.operator_id,
            cmd.payment_intent_id,
            cmd.acquirer_link_id,
            cmd.reason_code,
            cmd.amount_minor_units,
            cmd.currency,
        )?;

        self.repo.save(&case).await?;
        Ok(case)
    }

    async fn submit_representment(&self, cmd: SubmitRepresentmentCommand) -> Result<ChargebackCase, DisputeError> {
        let mut case = self.repo.load(cmd.chargeback_id).await?
            .ok_or(DisputeError::NotFound(cmd.chargeback_id))?;

        case.submit_representment(cmd.evidence)?;
        self.repo.save(&case).await?;
        Ok(case)
    }

    async fn resolve_chargeback(&self, cmd: ResolveChargebackCommand) -> Result<ChargebackCase, DisputeError> {
        let mut case = self.repo.load(cmd.chargeback_id).await?
            .ok_or(DisputeError::NotFound(cmd.chargeback_id))?;

        case.resolve(cmd.outcome, cmd.resolution_note)?;
        self.repo.save(&case).await?;
        Ok(case)
    }
}
