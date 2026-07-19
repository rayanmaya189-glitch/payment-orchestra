use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::api::dto::PaymentIntentResponse;
use crate::domain::aggregates::{PaymentIntent, RoutingAttempt, RoutingPolicy};
use crate::domain::value_objects::PaymentPurpose;
use crate::infrastructure::repository::{PaymentIntentRepository, RoutingPolicyRepository};
use platform_error::{PlatformError, ConflictError};
use shared_types::{Money, PaymentStatus, ActorType};
use shared_types::events::EventEnvelope;

#[async_trait]
pub trait PaymentService: Send + Sync {
    async fn create_payment_intent(&self, cmd: CreatePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError>;
    async fn authorize(&self, cmd: AuthorizePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError>;
    async fn capture(&self, cmd: CapturePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError>;
    async fn void(&self, cmd: VoidPaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError>;
    async fn refund(&self, cmd: RefundPaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError>;
    async fn get_payment_intent(&self, query: GetPaymentIntentQuery) -> Result<PaymentIntentResponse, PlatformError>;
    async fn list_payment_intents(&self, query: ListPaymentIntentsQuery) -> Result<Vec<PaymentIntentResponse>, PlatformError>;
}

pub struct PaymentServiceImpl {
    intent_repo: Box<dyn PaymentIntentRepository>,
    policy_repo: Box<dyn RoutingPolicyRepository>,
    db: sea_orm::DatabaseConnection,
}

impl PaymentServiceImpl {
    pub fn new(
        intent_repo: Box<dyn PaymentIntentRepository>,
        policy_repo: Box<dyn RoutingPolicyRepository>,
        db: sea_orm::DatabaseConnection,
    ) -> Self {
        Self { intent_repo, policy_repo, db }
    }
}

#[async_trait]
impl PaymentService for PaymentServiceImpl {
    async fn create_payment_intent(&self, cmd: CreatePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // Check idempotency
        if let Some(existing) = self.intent_repo.find_by_idempotency_key(&cmd.idempotency_key).await? {
            return Ok(intent_to_response(&existing));
        }

        let purpose = match cmd.purpose.as_deref() {
            Some("card_verification") => PaymentPurpose::CardVerification,
            _ => PaymentPurpose::Payment,
        };

        let mut intent = PaymentIntent::new(
            cmd.operator_id,
            cmd.amount.clone(),
            cmd.idempotency_key,
            purpose,
        );

        intent.metadata = cmd.metadata;
        intent.gateway_profile_id = cmd.preferred_gateway_profile_id;

        self.intent_repo.save(&intent).await?;

        // Publish event
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "PaymentIntent",
            intent.payment_intent_id,
            "PaymentIntentCreated",
            ActorType::System.as_str(),
            correlation_id,
            serde_json::json!({
                "payment_intent_id": intent.payment_intent_id,
                "operator_id": intent.operator_id,
                "amount": intent.requested_amount.amount_minor_units,
                "currency": intent.requested_amount.currency.0,
                "status": intent.status.as_str(),
            }),
        );

        // TODO: Publish to NATS

        Ok(intent_to_response(&intent))
    }

    async fn authorize(&self, cmd: AuthorizePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        let mut intent = self.intent_repo
            .load(cmd.payment_intent_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "PaymentIntent".into(),
                id: cmd.payment_intent_id,
            })?;

        if !intent.can_authorize() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: intent.status.as_str().to_string(),
                    command: "Authorize".to_string(),
                }
            ));
        }

        // Load routing policy
        let policy = self.policy_repo
            .load_active_for_operator(intent.operator_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "RoutingPolicy".into(),
                id: intent.operator_id,
            })?;

        // Find attempted links
        let attempts = self.intent_repo.load_attempts(intent.payment_intent_id).await?;
        let attempted_links: Vec<Uuid> = attempts.iter().map(|a| a.acquirer_link_id).collect();

        // Select route
        let selected_link = policy.select_route(
            None, // TODO: Get card scheme from payment method
            &intent.requested_amount.currency,
            &intent.requested_amount,
            &attempted_links,
        );

        let acquirer_link_id = match selected_link {
            Some(id) => id,
            None => {
                intent.record_failure();
                self.intent_repo.save(&intent).await?;
                return Ok(intent_to_response(&intent));
            }
        };

        // Create routing attempt
        let attempt_number = (attempts.len() + 1) as i32;
        let mut attempt = RoutingAttempt::new(
            intent.payment_intent_id,
            attempt_number,
            acquirer_link_id,
            "unknown".to_string(), // TODO: Get connector_id from link
        );

        // TODO: Actually call the connector gateway
        // For now, simulate success
        attempt.status = "approved".to_string();
        attempt.acquirer_reference = Some(format!("acq_{}", Uuid::now_v7()));
        attempt.latency_ms = 150;

        self.intent_repo.save_attempt(&attempt).await?;

        // Record authorization
        intent.record_authorization(intent.requested_amount.clone());
        intent.payment_method_token_id = Some(cmd.payment_method_token_id);
        self.intent_repo.save(&intent).await?;

        Ok(intent_to_response(&intent))
    }

    async fn capture(&self, cmd: CapturePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        let mut intent = self.intent_repo
            .load(cmd.payment_intent_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "PaymentIntent".into(),
                id: cmd.payment_intent_id,
            })?;

        if !intent.can_capture() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: intent.status.as_str().to_string(),
                    command: "Capture".to_string(),
                }
            ));
        }

        let capture_amount = match cmd.amount {
            Some(amount) => amount,
            None => intent.requested_amount.clone(),
        };

        // Validate capture amount
        if capture_amount.amount_minor_units > intent.remaining_capture_amount() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(
                    "Capture amount exceeds authorized amount".into()
                )
            ));
        }

        // Calculate new captured amount
        let new_captured = Money {
            amount_minor_units: intent.captured_amount.amount_minor_units + capture_amount.amount_minor_units,
            currency: intent.captured_amount.currency.clone(),
        };

        intent.record_capture(new_captured);
        self.intent_repo.save(&intent).await?;

        Ok(intent_to_response(&intent))
    }

    async fn void(&self, cmd: VoidPaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        let mut intent = self.intent_repo
            .load(cmd.payment_intent_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "PaymentIntent".into(),
                id: cmd.payment_intent_id,
            })?;

        if !intent.can_void() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: intent.status.as_str().to_string(),
                    command: "Void".to_string(),
                }
            ));
        }

        intent.record_void();
        self.intent_repo.save(&intent).await?;

        Ok(intent_to_response(&intent))
    }

    async fn refund(&self, cmd: RefundPaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        let mut intent = self.intent_repo
            .load(cmd.payment_intent_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "PaymentIntent".into(),
                id: cmd.payment_intent_id,
            })?;

        if !intent.can_refund() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: intent.status.as_str().to_string(),
                    command: "Refund".to_string(),
                }
            ));
        }

        if cmd.amount.amount_minor_units > intent.remaining_refund_amount() {
            return Err(PlatformError::Conflict(ConflictError::FullyRefunded));
        }

        intent.record_refund(cmd.amount);
        self.intent_repo.save(&intent).await?;

        Ok(intent_to_response(&intent))
    }

    async fn get_payment_intent(&self, query: GetPaymentIntentQuery) -> Result<PaymentIntentResponse, PlatformError> {
        let intent = self.intent_repo
            .load(query.payment_intent_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "PaymentIntent".into(),
                id: query.payment_intent_id,
            })?;

        Ok(intent_to_response(&intent))
    }

    async fn list_payment_intents(&self, query: ListPaymentIntentsQuery) -> Result<Vec<PaymentIntentResponse>, PlatformError> {
        // TODO: Implement with proper filtering
        Ok(Vec::new())
    }
}

fn intent_to_response(intent: &PaymentIntent) -> PaymentIntentResponse {
    PaymentIntentResponse {
        payment_intent_id: intent.payment_intent_id,
        operator_id: intent.operator_id,
        status: intent.status.as_str().to_string(),
        amount: intent.requested_amount.amount_minor_units,
        currency: intent.requested_amount.currency.0.clone(),
        authorized_amount: intent.authorized_amount.amount_minor_units,
        captured_amount: intent.captured_amount.amount_minor_units,
        refunded_amount: intent.refunded_amount.amount_minor_units,
        idempotency_key: intent.idempotency_key.clone(),
        created_at: intent.created_at.to_rfc3339(),
        updated_at: intent.updated_at.to_rfc3339(),
    }
}
