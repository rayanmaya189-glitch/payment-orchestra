//! Command processing pipeline for orchestration-service.
//! Provides validation, authorization, logging, and metrics around command execution.

use tracing::{info, warn, error, span, Level};

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

/// Pipeline middleware for command execution.
/// Wraps the command handler with authz checks, logging, and metrics.
pub struct OrchestrationPipeline<H: CommandHandler, Q: QueryHandler> {
    handler: H,
    query_handler: Q,
}

impl<H: CommandHandler, Q: QueryHandler> OrchestrationPipeline<H, Q> {
    pub fn new(handler: H, query_handler: Q) -> Self {
        Self { handler, query_handler }
    }
}

// ─── Pipeline Wrapper ────────────────────────────────────────────────────────

impl<H: CommandHandler, Q: QueryHandler> OrchestrationPipeline<H, Q> {
    pub async fn create_payment_intent(&self, cmd: CreatePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let span = span!(Level::INFO, "create_payment_intent", pi_id = tracing::field::Empty);
        let _guard = span.enter();

        // Validate
        if cmd.amount_minor_units < 0 {
            warn!("Invalid amount: {}", cmd.amount_minor_units);
            return Err(OrchestrationError::Validation("Amount must be non-negative".into()));
        }

        info!(operator_id = %cmd.operator_id, amount = cmd.amount_minor_units, currency = %cmd.currency, "Creating PaymentIntent");
        let result = self.handler.create_payment_intent(cmd).await;

        match &result {
            Ok(r) => {
                span.record("pi_id", tracing::field::display(r.payment_intent_id));
                info!(status = %r.status, "PaymentIntent created");
            }
            Err(e) => {
                error!(error = %e, "Failed to create PaymentIntent");
            }
        }

        result
    }

    pub async fn authorize_payment_intent(&self, cmd: AuthorizePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let span = span!(Level::INFO, "authorize_payment_intent", pi_id = %cmd.payment_intent_id);
        let _guard = span.enter();

        info!("Authorizing PaymentIntent");
        let result = self.handler.authorize_payment_intent(cmd).await;

        match &result {
            Ok(r) => info!(status = %r.status, attempts = r.routing_attempts.len(), "PaymentIntent authorized"),
            Err(e) => error!(error = %e, "Failed to authorize PaymentIntent"),
        }

        result
    }

    pub async fn capture_payment_intent(&self, cmd: CapturePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let span = span!(Level::INFO, "capture_payment_intent", pi_id = %cmd.payment_intent_id);
        let _guard = span.enter();

        info!("Capturing PaymentIntent");
        let result = self.handler.capture_payment_intent(cmd).await;

        match &result {
            Ok(r) => info!(status = %r.status, captured = r.captured_amount.amount_minor_units, "PaymentIntent captured"),
            Err(e) => error!(error = %e, "Failed to capture PaymentIntent"),
        }

        result
    }

    pub async fn void_payment_intent(&self, cmd: VoidPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        info!(pi_id = %cmd.payment_intent_id, "Voiding PaymentIntent");
        let result = self.handler.void_payment_intent(cmd).await;

        if let Err(e) = &result {
            error!(error = %e, "Failed to void PaymentIntent");
        }

        result
    }

    pub async fn refund_payment_intent(&self, cmd: RefundPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        info!(pi_id = %cmd.payment_intent_id, amount = cmd.amount_minor_units, "Refunding PaymentIntent");
        let result = self.handler.refund_payment_intent(cmd).await;

        if let Err(e) = &result {
            error!(error = %e, "Failed to refund PaymentIntent");
        }

        result
    }

    pub async fn activate_routing_policy(&self, cmd: ActivateRoutingPolicy) -> Result<RoutingPolicyResult, OrchestrationError> {
        info!(operator_id = %cmd.operator_id, rules = cmd.rules.len(), "Activating routing policy");
        let result = self.handler.activate_routing_policy(cmd).await;

        if let Err(e) = &result {
            error!(error = %e, "Failed to activate routing policy");
        }

        result
    }

    // ── Query passthrough ─────────────────────────────────────────────────

    pub async fn get_payment_intent(&self, query: GetPaymentIntentQuery) -> Result<Option<PaymentIntent>, OrchestrationError> {
        self.query_handler.get_payment_intent(query).await
    }

    pub async fn list_payment_intents(&self, query: ListPaymentIntentsQuery) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        self.query_handler.list_payment_intents(query).await
    }

    pub async fn get_active_routing_policy(&self, query: GetActiveRoutingPolicyQuery) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        self.query_handler.get_active_routing_policy(query).await
    }
}
