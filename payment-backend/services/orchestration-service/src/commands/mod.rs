//! Command handlers for orchestration-service.
//!
//! Each command represents a mutating operation on the PaymentIntent, RoutingPolicy,
//! or PaymentMethodToken aggregate.

pub mod types;
pub(crate) mod payment;
pub(crate) mod routing;
pub(crate) mod token;

pub use types::*;

use async_trait::async_trait;

use crate::domain::*;
use crate::repository::*;

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
    pub(crate) repo: R,
}

impl<R: OrchestrationRepository> OrchestrationCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: OrchestrationRepository + Send + Sync> CommandHandler for OrchestrationCommandHandler<R> {
    async fn create_payment_intent(&self, cmd: CreatePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        self.create_payment_intent_impl(cmd).await
    }

    async fn authorize_payment_intent(&self, cmd: AuthorizePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        self.authorize_payment_intent_impl(cmd).await
    }

    async fn capture_payment_intent(&self, cmd: CapturePaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        self.capture_payment_intent_impl(cmd).await
    }

    async fn void_payment_intent(&self, cmd: VoidPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        self.void_payment_intent_impl(cmd).await
    }

    async fn refund_payment_intent(&self, cmd: RefundPaymentIntent) -> Result<PaymentIntentResult, OrchestrationError> {
        self.refund_payment_intent_impl(cmd).await
    }

    async fn activate_routing_policy(&self, cmd: ActivateRoutingPolicy) -> Result<RoutingPolicyResult, OrchestrationError> {
        self.activate_routing_policy_impl(cmd).await
    }

    async fn store_payment_method_token(&self, cmd: StorePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        self.store_payment_method_token_impl(cmd).await
    }

    async fn expire_payment_method_token(&self, cmd: ExpirePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        self.expire_payment_method_token_impl(cmd).await
    }

    async fn revoke_payment_method_token(&self, cmd: RevokePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        self.revoke_payment_method_token_impl(cmd).await
    }
}
