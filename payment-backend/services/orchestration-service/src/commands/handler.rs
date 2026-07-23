//! Command handler trait and implementation for orchestration-service.

use std::hash::{Hash, Hasher};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::events::*;
use super::types::*;

// ─── Command Handler Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_payment_intent(&self, cmd: CreatePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError>;
    async fn authorize_payment_intent(&self, cmd: AuthorizePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError>;
    async fn capture_payment_intent(&self, cmd: CapturePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError>;
    async fn void_payment_intent(&self, cmd: VoidPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError>;
    async fn refund_payment_intent(&self, cmd: RefundPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError>;
    async fn activate_routing_policy(&self, cmd: ActivateRoutingPolicy) -> Result<RoutingPolicyResult, OrchestrationError>;
    async fn store_payment_method_token(&self, cmd: StorePaymentMethodToken) -> Result<TokenResult, OrchestrationError>;
    async fn expire_payment_method_token(&self, cmd: ExpirePaymentMethodToken) -> Result<TokenResult, OrchestrationError>;
    async fn revoke_payment_method_token(&self, cmd: RevokePaymentMethodToken) -> Result<TokenResult, OrchestrationError>;
}

// ─── Handler Implementation ──────────────────────────────────────────────────

pub struct OrchestrationCommandHandler<R: OrchestrationRepository> {
    repo: R,
}

impl<R: OrchestrationRepository> OrchestrationCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: OrchestrationRepository + Send + Sync> CommandHandler for OrchestrationCommandHandler<R> {
    async fn create_payment_intent(&self, cmd: CreatePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let idem_key = format!("create_pi:{}", cmd.idempotency_key);
        match self.repo.check_idempotency(&idem_key).await? {
            IdempotencyResult::Duplicate(result) => {
                return serde_json::from_value(result)
                    .map_err(|_| OrchestrationError::Validation("Idempotency cache deserialization failed".into()));
            }
            IdempotencyResult::Conflict => {
                return Err(OrchestrationError::IdempotencyConflict(cmd.idempotency_key));
            }
            IdempotencyResult::New => {}
        }

        if cmd.amount_minor_units < 0 {
            return Err(OrchestrationError::Validation("Amount must be non-negative".into()));
        }
        if cmd.currency.len() != 3 {
            return Err(OrchestrationError::Validation("Invalid currency code".into()));
        }

        let payment_intent_id = Uuid::now_v7();
        let amount = Money { amount_minor_units: cmd.amount_minor_units, currency: cmd.currency.clone() };

        let mut intent = PaymentIntent::new(
            payment_intent_id,
            cmd.operator_id,
            amount,
            cmd.purpose.clone(),
            cmd.idempotency_key.clone(),
            cmd.source_type,
            cmd.source_id,
            cmd.metadata.clone(),
        );

        let created_event = PaymentIntentCreated {
            payment_intent_id,
            operator_id: cmd.operator_id,
            amount_minor_units: cmd.amount_minor_units,
            currency: cmd.currency,
            idempotency_key: cmd.idempotency_key,
            is_card_verification: cmd.purpose == PaymentPurpose::CardVerification,
            source_type: intent.source_type.clone(),
            source_id: intent.source_id,
            metadata: cmd.metadata,
            occurred_at: Utc::now(),
        };

        let events = vec![PaymentEvent::PaymentIntentCreated(created_event)];

        for event in &events {
            intent.apply_event(event);
        }

        let result = PaymentIntentResult {
            payment_intent_id,
            status: intent.status.clone(),
            requested_amount: intent.requested_amount.clone(),
            authorized_amount: intent.authorized_amount.clone(),
            captured_amount: intent.captured_amount.clone(),
            refunded_amount: intent.refunded_amount.clone(),
            routing_attempts: intent.routing_attempts.clone(),
            events: events.clone(),
        };

        self.repo.save_payment_intent(&intent).await?;
        self.repo.store_idempotency(&idem_key, &serde_json::to_value(&result).unwrap_or_default()).await?;

        Ok(result)
    }

    async fn authorize_payment_intent(&self, cmd: AuthorizePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let mut intent = self.repo.load_payment_intent(cmd.payment_intent_id).await?
            .ok_or(OrchestrationError::NotFound(cmd.payment_intent_id))?;

        intent.status.can_execute_command("Authorize")?;

        let policy = self.repo.load_active_routing_policy(intent.operator_id).await?
            .ok_or(OrchestrationError::RoutingPolicyNotFound)?;

        let available_links = self.repo.list_active_acquirer_links(intent.operator_id).await?;
        if available_links.is_empty() {
            return Err(OrchestrationError::NoEligibleRoute);
        }

        let card_scheme = &cmd.card_scheme;
        let attempted_hops: Vec<Uuid> = intent.routing_attempts.iter()
            .map(|a| a.acquirer_link_id)
            .collect();

        let attempt_id = Uuid::now_v7();
        let attempt_number = (intent.routing_attempts.len() + 1) as i32;

        match policy.select_route(card_scheme, &intent.currency, intent.requested_amount.amount_minor_units, &attempted_hops, &available_links) {
            Ok(acquirer_link_id) => {
                let auth_event = PaymentAuthorized {
                    payment_intent_id: cmd.payment_intent_id,
                    acquirer_link_id,
                    acquirer_reference: format!("auth_{}", Uuid::now_v7()),
                    authorized_amount_minor: intent.requested_amount.amount_minor_units,
                    gateway_profile_id: None,
                    gateway_profile_version: None,
                    rotation_strategy: None,
                    selection_reason: Some("priority_1".into()),
                    expected_settlement_date: None,
                    settlement_cycle: None,
                    occurred_at: Utc::now(),
                };

                let attempt_evt = PaymentAuthorizationAttempted {
                    payment_intent_id: cmd.payment_intent_id,
                    attempt_id,
                    attempt_number,
                    acquirer_link_id,
                    connector_id: "mock_connector".into(),
                    declined: false,
                    decline_reason: None,
                    acquirer_reference: Some(auth_event.acquirer_reference.clone()),
                    latency_ms: 120,
                    occurred_at: Utc::now(),
                };

                let events = vec![
                    PaymentEvent::PaymentAuthorizationAttempted(attempt_evt),
                    PaymentEvent::PaymentAuthorized(auth_event),
                ];

                for event in &events {
                    intent.apply_event(event);
                }

                intent.routing_attempts.push(RoutingAttempt {
                    attempt_id,
                    payment_intent_id: cmd.payment_intent_id,
                    attempt_number,
                    acquirer_link_id,
                    gateway_profile_id: None,
                    gateway_profile_snapshot: None,
                    connector_id: "mock_connector".into(),
                    status: AttemptStatus::Approved,
                    decline_reason: None,
                    acquirer_reference: Some(format!("auth_ref_{}", Uuid::now_v7())),
                    latency_ms: 120,
                    fee_calculated: None,
                    attempted_at: Utc::now(),
                });

                self.repo.save_payment_intent(&intent).await?;

                Ok(PaymentIntentResult {
                    payment_intent_id: cmd.payment_intent_id,
                    status: intent.status,
                    requested_amount: intent.requested_amount,
                    authorized_amount: intent.authorized_amount,
                    captured_amount: intent.captured_amount,
                    refunded_amount: intent.refunded_amount,
                    routing_attempts: intent.routing_attempts.clone(),
                    events,
                })
            }
            Err(_) if attempted_hops.is_empty() => {
                Err(OrchestrationError::NoEligibleRoute)
            }
            Err(_) => {
                let attempt_evt = PaymentAuthorizationAttempted {
                    payment_intent_id: cmd.payment_intent_id,
                    attempt_id,
                    attempt_number,
                    acquirer_link_id: Uuid::nil(),
                    connector_id: "unknown".into(),
                    declined: true,
                    decline_reason: Some("NoEligibleRoute".into()),
                    acquirer_reference: None,
                    latency_ms: 0,
                    occurred_at: Utc::now(),
                };

                let failed_evt = PaymentFailedAllRoutes {
                    payment_intent_id: cmd.payment_intent_id,
                    attempts: intent.routing_attempts.iter().map(|a| FailedAttemptInfo {
                        attempt_number: a.attempt_number,
                        acquirer_link_id: a.acquirer_link_id,
                        decline_reason: a.decline_reason.as_ref().map(|d| d.to_string()).unwrap_or_else(|| "Unknown".into()),
                    }).collect(),
                    occurred_at: Utc::now(),
                };

                let events = vec![
                    PaymentEvent::PaymentAuthorizationAttempted(attempt_evt),
                    PaymentEvent::PaymentFailedAllRoutes(failed_evt),
                ];

                for event in &events {
                    intent.apply_event(event);
                }
                self.repo.save_payment_intent(&intent).await?;

                Ok(PaymentIntentResult {
                    payment_intent_id: cmd.payment_intent_id,
                    status: intent.status,
                    requested_amount: intent.requested_amount,
                    authorized_amount: intent.authorized_amount,
                    captured_amount: intent.captured_amount,
                    refunded_amount: intent.refunded_amount,
                    routing_attempts: intent.routing_attempts.clone(),
                    events,
                })
            }
        }
    }

    async fn capture_payment_intent(&self, cmd: CapturePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let mut intent = self.repo.load_payment_intent(cmd.payment_intent_id).await?
            .ok_or(OrchestrationError::NotFound(cmd.payment_intent_id))?;

        intent.status.can_execute_command("Capture")?;

        let capture_amount = match cmd.amount_minor_units {
            Some(amount) => Money { amount_minor_units: amount, currency: intent.currency.clone() },
            None => intent.authorized_amount.clone(),
        };

        intent.check_capture_invariant(&capture_amount)?;

        if cmd.amount_minor_units.is_some() && !cmd.supports_partial_capture {
            return Err(OrchestrationError::InvalidStateTransition("PARTIAL_CAPTURE_NOT_SUPPORTED".into()));
        }

        if cmd.amount_minor_units.is_some() {
            let existing_partial_captures = intent.routing_attempts.iter()
                .filter(|a| matches!(a.status, AttemptStatus::Approved))
                .count() as u32;
            if existing_partial_captures >= cmd.max_partial_captures {
                return Err(OrchestrationError::InvalidStateTransition("MAX_PARTIAL_CAPTURES_EXCEEDED".into()));
            }
        }

        let acquirer_ref = format!("cap_{}", Uuid::now_v7());

        let events = if capture_amount.amount_minor_units == intent.authorized_amount.amount_minor_units {
            vec![PaymentEvent::PaymentCaptured(PaymentCaptured {
                payment_intent_id: cmd.payment_intent_id,
                captured_amount_minor: capture_amount.amount_minor_units,
                acquirer_reference: acquirer_ref,
                occurred_at: Utc::now(),
            })]
        } else {
            let remaining = intent.authorized_amount.checked_sub(&capture_amount)
                .map_err(|_| OrchestrationError::Validation("Capture exceeds authorized".into()))?;
            vec![PaymentEvent::PaymentPartiallyCaptured(PaymentPartiallyCaptured {
                payment_intent_id: cmd.payment_intent_id,
                captured_amount_minor: capture_amount.amount_minor_units,
                remaining_authorized_minor: remaining.amount_minor_units,
                acquirer_reference: acquirer_ref,
                occurred_at: Utc::now(),
            })]
        };

        for event in &events {
            intent.apply_event(event);
        }

        self.repo.save_payment_intent(&intent).await?;

        Ok(PaymentIntentResult {
            payment_intent_id: cmd.payment_intent_id,
            status: intent.status,
            requested_amount: intent.requested_amount,
            authorized_amount: intent.authorized_amount,
            captured_amount: intent.captured_amount,
            refunded_amount: intent.refunded_amount,
            routing_attempts: intent.routing_attempts.clone(),
            events,
        })
    }

    async fn void_payment_intent(&self, cmd: VoidPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let mut intent = self.repo.load_payment_intent(cmd.payment_intent_id).await?
            .ok_or(OrchestrationError::NotFound(cmd.payment_intent_id))?;

        intent.status.can_execute_command("Void")?;

        let acquirer_ref = intent.routing_attempts.first()
            .and_then(|a| a.acquirer_reference.clone())
            .unwrap_or_else(|| format!("void_{}", Uuid::now_v7()));

        let events = vec![PaymentEvent::PaymentVoided(PaymentVoided {
            payment_intent_id: cmd.payment_intent_id,
            acquirer_reference: acquirer_ref,
            occurred_at: Utc::now(),
        })];

        for event in &events {
            intent.apply_event(event);
        }

        self.repo.save_payment_intent(&intent).await?;

        Ok(PaymentIntentResult {
            payment_intent_id: cmd.payment_intent_id,
            status: intent.status,
            requested_amount: intent.requested_amount,
            authorized_amount: intent.authorized_amount,
            captured_amount: intent.captured_amount,
            refunded_amount: intent.refunded_amount,
            routing_attempts: intent.routing_attempts.clone(),
            events,
        })
    }

    async fn refund_payment_intent(&self, cmd: RefundPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        let mut intent = self.repo.load_payment_intent(cmd.payment_intent_id).await?
            .ok_or(OrchestrationError::NotFound(cmd.payment_intent_id))?;

        intent.status.can_execute_command("Refund")?;

        if cmd.amount_minor_units <= 0 {
            return Err(OrchestrationError::Validation("Refund amount must be positive".into()));
        }

        let refund_amount = Money { amount_minor_units: cmd.amount_minor_units, currency: intent.currency.clone() };
        let remaining = intent.remaining_refundable()?;

        if refund_amount.amount_minor_units > remaining.amount_minor_units {
            return Err(OrchestrationError::Validation("INSUFFICIENT_REFUNDABLE_BALANCE".into()));
        }

        let acquirer_ref = intent.routing_attempts.first()
            .and_then(|a| a.acquirer_reference.clone())
            .unwrap_or_else(|| format!("ref_{}", Uuid::now_v7()));

        let events = if refund_amount.amount_minor_units == remaining.amount_minor_units {
            vec![PaymentEvent::PaymentRefunded(PaymentRefunded {
                payment_intent_id: cmd.payment_intent_id,
                refund_amount_minor: cmd.amount_minor_units,
                acquirer_reference: acquirer_ref,
                occurred_at: Utc::now(),
            })]
        } else {
            let remaining_refundable = remaining.checked_sub(&refund_amount)
                .map_err(|_| OrchestrationError::Validation("Refund exceeds balance".into()))?;
            vec![PaymentEvent::PaymentPartiallyRefunded(PaymentPartiallyRefunded {
                payment_intent_id: cmd.payment_intent_id,
                refund_amount_minor: cmd.amount_minor_units,
                remaining_refundable_minor: remaining_refundable.amount_minor_units,
                acquirer_reference: acquirer_ref,
                occurred_at: Utc::now(),
            })]
        };

        for event in &events {
            intent.apply_event(event);
        }

        self.repo.save_payment_intent(&intent).await?;

        Ok(PaymentIntentResult {
            payment_intent_id: cmd.payment_intent_id,
            status: intent.status,
            requested_amount: intent.requested_amount,
            authorized_amount: intent.authorized_amount,
            captured_amount: intent.captured_amount,
            refunded_amount: intent.refunded_amount,
            routing_attempts: intent.routing_attempts.clone(),
            events,
        })
    }

    async fn activate_routing_policy(&self, cmd: ActivateRoutingPolicy) -> Result<RoutingPolicyResult, OrchestrationError> {
        let policy_id = Uuid::now_v7();

        if let Some(mut existing) = self.repo.load_active_routing_policy(cmd.operator_id).await? {
            existing.status = PolicyStatus::Inactive;
            self.repo.save_routing_policy(&existing).await?;
        }

        let mut policy = RoutingPolicy::new(policy_id, cmd.operator_id, cmd.rules);
        policy.failover_config = cmd.failover_config;
        policy.partial_auth_strategy = cmd.partial_auth_strategy;
        policy.rotation_strategy = cmd.rotation_strategy;
        policy.max_transaction_amount_minor = cmd.max_transaction_amount_minor;
        policy.status = PolicyStatus::Active;
        policy.activated_at = Some(Utc::now());

        let rules_str = format!("{:?}", policy.rules);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        rules_str.hash(&mut hasher);
        let rules_hash = format!("{:x}", hasher.finish());

        let event = PaymentEvent::RoutingPolicyActivated(RoutingPolicyActivated {
            routing_policy_id: policy_id,
            operator_id: cmd.operator_id,
            version: policy.version,
            rules_hash,
            occurred_at: Utc::now(),
        });

        self.repo.save_routing_policy(&policy).await?;

        Ok(RoutingPolicyResult {
            routing_policy_id: policy_id,
            version: policy.version,
            status: PolicyStatus::Active,
            event,
        })
    }

    async fn store_payment_method_token(&self, cmd: StorePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        let token_id = Uuid::now_v7();
        let token = PaymentMethodToken {
            token_id,
            operator_id: cmd.operator_id,
            payment_method_type: cmd.payment_method_type.clone(),
            last_four: cmd.last_four.clone(),
            card_brand: cmd.card_brand.clone(),
            expiry_month: cmd.expiry_month,
            expiry_year: cmd.expiry_year,
            token_status: TokenStatus::Active,
            acquirer_link_id: cmd.acquirer_link_id,
            acquirer_token_reference: cmd.acquirer_token_reference.clone(),
            encrypted_token: cmd.encrypted_token.clone(),
            created_at: Utc::now(),
            expires_at: cmd.expires_at,
            revoked_at: None,
            revocation_reason: None,
        };

        let event = PaymentEvent::PaymentMethodTokenStored(PaymentMethodTokenStored {
            token_id,
            operator_id: cmd.operator_id,
            payment_method_type: cmd.payment_method_type,
            last_four: cmd.last_four,
            card_brand: cmd.card_brand,
            acquirer_link_id: cmd.acquirer_link_id,
            occurred_at: Utc::now(),
        });

        self.repo.save_payment_method_token(&token).await?;

        Ok(TokenResult { token_id, event })
    }

    async fn expire_payment_method_token(&self, cmd: ExpirePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        let mut token = self.repo.load_payment_method_token(cmd.token_id).await?
            .ok_or(OrchestrationError::PaymentMethodTokenInvalid)?;
        token.token_status = TokenStatus::Expired;

        let event = PaymentEvent::PaymentMethodTokenExpired(PaymentMethodTokenExpired {
            token_id: cmd.token_id,
            last_four: token.last_four.clone(),
            acquirer_link_id: token.acquirer_link_id,
            occurred_at: Utc::now(),
        });

        self.repo.save_payment_method_token(&token).await?;

        Ok(TokenResult { token_id: cmd.token_id, event })
    }

    async fn revoke_payment_method_token(&self, cmd: RevokePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        let mut token = self.repo.load_payment_method_token(cmd.token_id).await?
            .ok_or(OrchestrationError::PaymentMethodTokenInvalid)?;
        token.token_status = TokenStatus::Revoked;
        token.revoked_at = Some(Utc::now());
        token.revocation_reason = cmd.reason.clone();

        let event = PaymentEvent::PaymentMethodTokenRevoked(PaymentMethodTokenRevoked {
            token_id: cmd.token_id,
            last_four: token.last_four.clone(),
            acquirer_link_id: token.acquirer_link_id,
            revocation_reason: cmd.reason,
            occurred_at: Utc::now(),
        });

        self.repo.save_payment_method_token(&token).await?;

        Ok(TokenResult { token_id: cmd.token_id, event })
    }
}
