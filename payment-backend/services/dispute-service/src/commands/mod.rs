//! Dispute Management command handlers — BC-10

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

/// Record a new chargeback from an acquirer notification.
pub struct RecordChargebackCommand {
    pub operator_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub reason_code: String,
    pub amount_minor_units: i64,
    pub currency: String,
    /// Whether the payment intent is in captured state (INV-08).
    pub is_captured: bool,
}

/// Submit representment evidence for a chargeback case.
pub struct SubmitRepresentmentCommand {
    pub chargeback_id: Uuid,
    pub evidence: RepresentmentEvidence,
}

/// Resolve a chargeback case with an outcome.
pub struct ResolveChargebackCommand {
    pub chargeback_id: Uuid,
    pub outcome: ChargebackOutcome,
    pub resolution_note: Option<String>,
}

// ---------------------------------------------------------------------------
// Command handler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn record_chargeback(
        &self,
        cmd: RecordChargebackCommand,
    ) -> Result<ChargebackCase, DisputeError>;

    async fn submit_representment(
        &self,
        cmd: SubmitRepresentmentCommand,
    ) -> Result<ChargebackCase, DisputeError>;

    async fn resolve_chargeback(
        &self,
        cmd: ResolveChargebackCommand,
    ) -> Result<ChargebackCase, DisputeError>;
}

// ---------------------------------------------------------------------------
// Handler implementation
// ---------------------------------------------------------------------------

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
    async fn record_chargeback(
        &self,
        cmd: RecordChargebackCommand,
    ) -> Result<ChargebackCase, DisputeError> {
        // INV-08: Must reference a Captured PaymentIntent
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

    async fn submit_representment(
        &self,
        cmd: SubmitRepresentmentCommand,
    ) -> Result<ChargebackCase, DisputeError> {
        let mut case = self
            .repo
            .load(cmd.chargeback_id)
            .await?
            .ok_or(DisputeError::NotFound(cmd.chargeback_id))?;

        case.submit_representment(cmd.evidence)?;
        self.repo.save(&case).await?;
        Ok(case)
    }

    async fn resolve_chargeback(
        &self,
        cmd: ResolveChargebackCommand,
    ) -> Result<ChargebackCase, DisputeError> {
        let mut case = self
            .repo
            .load(cmd.chargeback_id)
            .await?
            .ok_or(DisputeError::NotFound(cmd.chargeback_id))?;

        case.resolve(cmd.outcome, cmd.resolution_note)?;
        self.repo.save(&case).await?;
        Ok(case)
    }
}
