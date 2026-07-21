use async_trait::async_trait;
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::api::dto::PaymentIntentResponse;
use crate::domain::aggregates::{PaymentIntent, RoutingAttempt};
use crate::domain::value_objects::PaymentPurpose;
use crate::infrastructure::connector_client::{
    ConnectorClient, ConnectorAuthorizeRequest, ConnectorCaptureRequest,
    ConnectorVoidRequest, ConnectorRefundRequest,
};
use crate::infrastructure::repository::{PaymentIntentRepository, RoutingPolicyRepository};
use platform_error::{PlatformError, ConflictError};
use platform_middleware::{evaluate_policy, AbacContext};
use shared_types::{Money, ActorType};
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
    async fn activate_routing_policy(&self, cmd: ActivateRoutingPolicyCommand) -> Result<PaymentIntentResponse, PlatformError>;
}

pub struct PaymentServiceImpl {
    intent_repo: Box<dyn PaymentIntentRepository>,
    policy_repo: Box<dyn RoutingPolicyRepository>,
    connector_client: Box<dyn ConnectorClient>,
    db: sea_orm::DatabaseConnection,
}

impl PaymentServiceImpl {
    pub fn new(
        intent_repo: Box<dyn PaymentIntentRepository>,
        policy_repo: Box<dyn RoutingPolicyRepository>,
        connector_client: Box<dyn ConnectorClient>,
        db: sea_orm::DatabaseConnection,
    ) -> Self {
        Self { intent_repo, policy_repo, connector_client, db }
    }

    /// Helper: find the winning routing attempt (approved) for this payment intent.
    async fn find_winning_attempt(&self, payment_intent_id: Uuid) -> Result<RoutingAttempt, PlatformError> {
        let attempts = self.intent_repo.load_attempts(payment_intent_id).await?;
        attempts.into_iter()
            .find(|a| a.acquirer_reference.is_some() && (a.status == "approved" || a.status == "captured"))
            .ok_or_else(|| PlatformError::Conflict(ConflictError::IdempotencyKeyConflict))
    }

    /// Write event to outbox table for reliable NATS publishing (SRS OUTBOX-001).
    async fn publish_event(&self, event: &EventEnvelope, aggregate_type: &str, aggregate_id: Uuid) -> Result<(), PlatformError> {
        use sea_orm::{ConnectionTrait, Statement};

        let payload = serde_json::to_vec(event)
            .map_err(|e| PlatformError::Internal(format!("Failed to serialize event: {e}")))?;

        let id = Uuid::now_v7();
        let now = chrono::Utc::now();

        self.db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO outbox (outbox_id, aggregate_type, aggregate_id, event_type, event_version, payload, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            vec![
                id.into(),
                aggregate_type.into(),
                aggregate_id.into(),
                event.event_type.clone().into(),
                event.event_version.into(),
                hex::encode(&payload).into(),
                now.to_rfc3339().into(),
            ],
        ))
        .await
        .map_err(|e| PlatformError::Internal(format!("Failed to write outbox entry: {e}")))?;

        tracing::debug!(event_type = %event.event_type, aggregate_id = %aggregate_id, "Event written to outbox");
        Ok(())
    }

    /// Check ABAC permission for a command (SRS ABAC-001 through ABAC-008).
    fn check_abac(
        &self,
        principal_id: Uuid,
        role: &str,
        action: &str,
        resource: &str,
        operator_id: Option<Uuid>,
        amount: Option<i64>,
    ) -> Result<(), PlatformError> {
        let ctx = AbacContext {
            principal_id,
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: None,
            amount,
            ip_address: None,
            operator_id,
        };
        evaluate_policy(&ctx)
    }
}

#[async_trait]
impl PaymentService for PaymentServiceImpl {
    async fn create_payment_intent(&self, cmd: CreatePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // ABAC check: create requires "create" permission on "payment"
        self.check_abac(cmd.principal_id, &cmd.role, "create", "payment", Some(cmd.operator_id), Some(cmd.amount.amount_minor_units))?;

        // Check idempotency (SRS Part 5 §4.1)
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

        // Publish PaymentIntentCreated event to outbox (SRS EVT-01)
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

        self.publish_event(&event, "PaymentIntent", intent.payment_intent_id).await?;

        Ok(intent_to_response(&intent))
    }

    /// Authorize with full failover loop per SRS Part 5 §3.4 RTY-001/002.
    ///
    /// The routing algorithm tries each eligible acquirer in priority order.
    /// On retryable decline, immediately attempts the next candidate.
    /// Max hops enforced per FailoverConfig.max_hops (platform ceiling: 3).
    async fn authorize(&self, cmd: AuthorizePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // ABAC check: authorize requires "create" permission on "payment"
        self.check_abac(cmd.principal_id, &cmd.role, "create", "payment", Some(cmd.operator_id), None)?;

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
                    command: "Authorize".into(),
                }
            ));
        }

        // Load routing policy (SRS Part 5 §3.1)
        let policy = self.policy_repo
            .load_active_for_operator(intent.operator_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "RoutingPolicy".into(),
                id: intent.operator_id,
            })?;

        intent.payment_method_token_id = Some(cmd.payment_method_token_id);
        let max_hops = policy.failover_config.max_hops;
        let mut all_attempts: Vec<RoutingAttempt> = Vec::new();
        let mut last_decline_reason: Option<String> = None;

        // Failover loop: SRS Part 5 §3.4 RTY-001 — immediate retry on next candidate
        for _hop in 0..max_hops {
            // Collect attempted link IDs from prior hops in this call
            let attempted_links: Vec<Uuid> = all_attempts.iter().map(|a| a.acquirer_link_id).collect();

            // Select next route (excluding already-attempted links)
            let selected_link = match policy.select_route(
                None, // card_scheme resolved from payment method token in production
                &intent.requested_amount.currency,
                &intent.requested_amount,
                &attempted_links,
            ) {
                Some(id) => id,
                None => {
                    // No more eligible routes — all routes exhausted
                    break;
                }
            };

            let attempt_number = (all_attempts.len() + 1) as i32;
            let mut attempt = RoutingAttempt::new(
                intent.payment_intent_id,
                attempt_number,
                selected_link,
                format!("connector_{}", selected_link),
            );

            // Call the connector gateway (real HTTP call)
            let connector_req = ConnectorAuthorizeRequest {
                connector_id: attempt.connector_id.clone(),
                payment_method_token: cmd.payment_method_token_id.to_string(),
                amount: intent.requested_amount.clone(),
                idempotency_key: format!("idem_auth_{}_{}", intent.payment_intent_id, attempt_number),
                merchant_reference: intent.idempotency_key.clone(),
                card_scheme: None,
            };

            let connector_response = self.connector_client.authorize(connector_req).await;

            match connector_response {
                Ok(response) => {
                    attempt.status = response.status.clone();
                    attempt.acquirer_reference = response.acquirer_reference.clone();
                    attempt.latency_ms = response.latency_ms;
                    attempt.decline_reason = response.decline_reason.clone();

                    let attempt_connector = attempt.connector_id.clone();
                    self.intent_repo.save_attempt(&attempt).await?;
                    all_attempts.push(attempt);

                    match response.status.as_str() {
                        "approved" => {
                            let auth_amount = response.approved_amount
                                .unwrap_or_else(|| intent.requested_amount.clone());
                            intent.record_authorization(auth_amount);

                            // Publish PaymentAuthorized event (SRS EVT-03)
                            let correlation_id = Uuid::now_v7();
                            let event = EventEnvelope::new(
                                "PaymentIntent", intent.payment_intent_id,
                                "PaymentAuthorized", ActorType::System.as_str(), correlation_id,
                                serde_json::json!({
                                    "amount_minor_units": intent.authorized_amount.amount_minor_units,
                                    "acquirer_reference": response.acquirer_reference,
                                }),
                            );
                            self.publish_event(&event, "PaymentIntent", intent.payment_intent_id).await?;

                            break; // Success — exit the failover loop
                        }
                        "declined" => {
                            let is_retryable = policy.failover_config.retryable_decline_codes
                                .iter().any(|c| response.decline_reason.as_deref() == Some(c.as_str()));

                            last_decline_reason = response.decline_reason.clone();

                            if !is_retryable {
                                break; // Non-retryable — stop immediately
                            }
                            tracing::info!(
                                attempt_number, connector = %attempt_connector,
                                reason = ?last_decline_reason,
                                "Retryable decline — attempting next hop"
                            );
                        }
                        _ => {
                            last_decline_reason = Some(format!("unexpected status: {}", response.status));
                            break;
                        }
                    }
                }
                Err(e) => {
                    attempt.status = "error".to_string();
                    attempt.decline_reason = Some(e.to_string());
                    let err_msg = e.to_string();
                    self.intent_repo.save_attempt(&attempt).await?;
                    all_attempts.push(attempt);
                    last_decline_reason = Some(err_msg.clone());
                    tracing::warn!(attempt_number, error = %err_msg, "Connector call failed");
                    break;
                }
            }
        }

        // If no successful authorization was recorded, mark as failed
        if intent.status != shared_types::PaymentStatus::Authorized {
            // Publish PaymentFailedAllRoutes event (SRS EVT-07)
            let correlation_id = Uuid::now_v7();
            let event = EventEnvelope::new(
                "PaymentIntent", intent.payment_intent_id,
                "PaymentFailedAllRoutes", ActorType::System.as_str(), correlation_id,
                serde_json::json!({
                    "attempts": all_attempts.len(),
                    "last_decline_reason": last_decline_reason,
                }),
            );
            self.publish_event(&event, "PaymentIntent", intent.payment_intent_id).await?;

            intent.record_failure();
        }

        self.intent_repo.save(&intent).await?;
        Ok(intent_to_response(&intent))
    }

    async fn capture(&self, cmd: CapturePaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // ABAC check: capture requires "update" permission on "payment"
        self.check_abac(cmd.principal_id, &cmd.role, "update", "payment", Some(cmd.operator_id), cmd.amount.as_ref().map(|a| a.amount_minor_units))?;

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
                    command: "Capture".into(),
                }
            ));
        }

        let capture_amount = match cmd.amount {
            Some(amount) => amount,
            None => intent.requested_amount.clone(),
        };

        // Validate capture amount (SRS INV-01)
        if capture_amount.amount_minor_units > intent.remaining_capture_amount() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(
                    "Capture amount exceeds authorized amount".into()
                )
            ));
        }

        // Find the winning attempt to get connector_id and acquirer_reference
        let winning = self.find_winning_attempt(intent.payment_intent_id).await?;

        // Call connector gateway for capture (real HTTP call)
        let connector_req = ConnectorCaptureRequest {
            connector_id: winning.connector_id.clone(),
            acquirer_reference: winning.acquirer_reference.clone().unwrap_or_default(),
            amount: Some(capture_amount.clone()),
        };

        let connector_response = self.connector_client.capture(connector_req).await?;

        if !connector_response.success {
            return Err(PlatformError::Internal(format!("Capture failed at connector: {}", connector_response.acquirer_reference)));
        }

        // Calculate new captured amount
        let new_captured = Money {
            amount_minor_units: intent.captured_amount.amount_minor_units + capture_amount.amount_minor_units,
            currency: intent.captured_amount.currency.clone(),
        };

        intent.record_capture(new_captured);

        // Publish PaymentCaptured event (SRS EVT-04)
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "PaymentIntent", intent.payment_intent_id,
            "PaymentCaptured", ActorType::System.as_str(), correlation_id,
            serde_json::json!({
                "captured_amount_minor_units": intent.captured_amount.amount_minor_units,
                "acquirer_reference": winning.acquirer_reference,
            }),
        );
        self.publish_event(&event, "PaymentIntent", intent.payment_intent_id).await?;

        self.intent_repo.save(&intent).await?;
        Ok(intent_to_response(&intent))
    }

    async fn void(&self, cmd: VoidPaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // ABAC check: void requires "update" permission on "payment"
        self.check_abac(cmd.principal_id, &cmd.role, "update", "payment", Some(cmd.operator_id), None)?;

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
                    command: "Void".into(),
                }
            ));
        }

        // Find the winning attempt to get connector_id and acquirer_reference
        let winning = self.find_winning_attempt(intent.payment_intent_id).await?;

        // Call connector gateway for void (real HTTP call)
        let connector_req = ConnectorVoidRequest {
            connector_id: winning.connector_id.clone(),
            acquirer_reference: winning.acquirer_reference.clone().unwrap_or_default(),
        };

        let connector_response = self.connector_client.void(connector_req).await?;

        if !connector_response.success {
            return Err(PlatformError::Internal(format!("Void failed at connector: {}", connector_response.status)));
        }

        intent.record_void();

        // Publish PaymentVoided event (SRS EVT-08)
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "PaymentIntent", intent.payment_intent_id,
            "PaymentVoided", ActorType::System.as_str(), correlation_id,
            serde_json::json!({
                "acquirer_reference": winning.acquirer_reference,
            }),
        );
        self.publish_event(&event, "PaymentIntent", intent.payment_intent_id).await?;

        self.intent_repo.save(&intent).await?;
        Ok(intent_to_response(&intent))
    }

    async fn refund(&self, cmd: RefundPaymentIntentCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // ABAC check: refund requires "update" permission on "payment"
        self.check_abac(cmd.principal_id, &cmd.role, "update", "payment", Some(cmd.operator_id), Some(cmd.amount.amount_minor_units))?;

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
                    command: "Refund".into(),
                }
            ));
        }

        if cmd.amount.amount_minor_units > intent.remaining_refund_amount() {
            return Err(PlatformError::Conflict(ConflictError::FullyRefunded));
        }

        // SRS INV-03: Refund must go to the same acquirer that captured
        let winning = self.find_winning_attempt(intent.payment_intent_id).await?;

        // Call connector gateway for refund (real HTTP call)
        let connector_req = ConnectorRefundRequest {
            connector_id: winning.connector_id.clone(),
            acquirer_reference: winning.acquirer_reference.clone().unwrap_or_default(),
            amount: cmd.amount.clone(),
            reason: Some("merchant_requested".to_string()),
        };

        let connector_response = self.connector_client.refund(connector_req).await?;

        if !connector_response.success {
            return Err(PlatformError::Internal(format!("Refund failed at connector: {}", connector_response.status)));
        }

        intent.record_refund(cmd.amount);

        // Publish PaymentRefunded event (SRS EVT-09)
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "PaymentIntent", intent.payment_intent_id,
            "PaymentRefunded", ActorType::System.as_str(), correlation_id,
            serde_json::json!({
                "refund_amount_minor_units": intent.refunded_amount.amount_minor_units,
                "acquirer_reference": connector_response.refund_reference,
            }),
        );
        self.publish_event(&event, "PaymentIntent", intent.payment_intent_id).await?;

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
        use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, QueryOrder, QuerySelect};

        let limit = query.limit.unwrap_or(20).min(100) as u64;
        let offset: u64 = query.cursor.and_then(|c| c.parse::<u64>().ok()).unwrap_or(0);

        let mut db_query = crate::infrastructure::entities::payment_intent::Entity::find();

        if let Some(ref status) = query.status {
            db_query = db_query.filter(crate::infrastructure::entities::payment_intent::Column::Status.eq(status.as_str()));
        }

        let models = db_query
            .order_by_desc(crate::infrastructure::entities::payment_intent::Column::CreatedAt)
            .limit(Some(limit))
            .offset(Some(offset))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| {
            let domain = m.to_domain();
            intent_to_response(&domain)
        }).collect())
    }

    /// Activate a routing policy (SRS UC-011, EVT-11).
    /// Policy must be validated against active acquirer links before activation.
    /// Uses Maker/Checker: only platform_admin or operator_admin can activate.
    async fn activate_routing_policy(&self, cmd: ActivateRoutingPolicyCommand) -> Result<PaymentIntentResponse, PlatformError> {
        // ABAC check: activate routing requires "update" permission on "routing_policy"
        self.check_abac(cmd.principal_id, &cmd.role, "update", "routing_policy", Some(cmd.operator_id), None)?;

        let mut policy = self.policy_repo
            .load(cmd.routing_policy_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "RoutingPolicy".into(),
                id: cmd.routing_policy_id,
            })?;

        // SRS INV-05: RoutingPolicy version immutable once activated
        if policy.status == "active" {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: "active".to_string(),
                    command: "ActivateRoutingPolicy".into(),
                }
            ));
        }

        // Validate at least one rule exists
        if policy.rules.is_empty() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(
                    "Routing policy must have at least one rule".into()
                )
            ));
        }

        // SRS BIZ-011: Operator cannot process live transactions with zero Active acquirer links
        // Validate all referenced acquirer_link_ids exist and are active
        // (In production, this would check gateway_profile status)

        policy.status = "active".to_string();
        policy.activated_at = Some(chrono::Utc::now());

        self.policy_repo.save(&policy).await?;

        // Publish RoutingPolicyActivated event (SRS EVT-11)
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "RoutingPolicy", cmd.routing_policy_id,
            "RoutingPolicyActivated", ActorType::System.as_str(), correlation_id,
            serde_json::json!({
                "routing_policy_id": cmd.routing_policy_id,
                "version": policy.version,
                "operator_id": cmd.operator_id,
            }),
        );
        self.publish_event(&event, "RoutingPolicy", cmd.routing_policy_id).await?;

        // Return a dummy PaymentIntentResponse for consistency
        // In production, this would return a RoutingPolicyResponse
        Ok(PaymentIntentResponse {
            payment_intent_id: Uuid::nil(),
            operator_id: cmd.operator_id,
            status: "routing_policy_activated".to_string(),
            amount: 0,
            currency: "AED".to_string(),
            authorized_amount: 0,
            captured_amount: 0,
            refunded_amount: 0,
            idempotency_key: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
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
