//! orchestration-service — Payment Orchestration Core.
//!
//! This is the central domain service of the platform. It owns:
//! - **PaymentIntent** aggregate (event-sourced): Create, Authorize, Capture, Void, Refund
//! - **RoutingPolicy** aggregate (CRUD + events): Routing rules, failover, rotation strategies
//! - **PaymentMethodToken** aggregate (CRUD + events): Acquirer-issued token lifecycle
//!
//! ## Architecture Context
//! This module runs within the modular monolith alongside all other modules.
//! All inter-module communication uses in-process gRPC (synchronous) or
//! in-process NATS channels (asynchronous). The module boundaries defined here
//! can be extracted into separate microservices in a future architecture evolution.
//!
//! ## Pure Router
//! The platform is a routing and orchestration layer only. Funds flow directly
//! between the customer, the payment gateway, and the merchant bank account.
//! The platform never holds, touches, or controls funds.

pub mod domain;
pub mod commands;
pub mod queries;
pub mod events;
pub mod entities;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types for convenience
pub use domain::{
    PaymentIntent, PaymentStatus, RoutingPolicy, RoutingRule, RoutingAttempt,
    PaymentMethodToken, TokenStatus, PolicyStatus,
    Money, DeclineReason, PaymentPurpose, SourceType,
    AttemptStatus, OrchestrationError, IdempotencyResult,
};
pub use commands::{
    CommandHandler, OrchestrationCommandHandler,
    CreatePaymentIntent, AuthorizePaymentIntent,
    CapturePaymentIntent, VoidPaymentIntent, RefundPaymentIntent,
    ActivateRoutingPolicy,
    StorePaymentMethodToken, ExpirePaymentMethodToken, RevokePaymentMethodToken,
    PaymentIntentResult,
};
pub use events::{
    PaymentEvent,
    PaymentIntentCreated, PaymentAuthorized, PaymentCaptured,
    PaymentPartiallyCaptured, PaymentFailed, PaymentFailedAllRoutes,
    PaymentVoided, PaymentRefunded, PaymentPartiallyRefunded,
    RoutingPolicyActivated, RoutingPolicyDeactivated,
    PaymentMethodTokenStored, PaymentMethodTokenExpired, PaymentMethodTokenRevoked,
    RiskScoreAssigned, GatewayProfileSelected,
};
pub use repository::{
    PaymentIntentRepository, RoutingPolicyRepository,
    PaymentMethodTokenRepository, IdempotencyCache,
    AcquirerLinkProvider, OrchestrationRepository,
    InMemoryOrchestrationRepository,
};
pub use queries::{
    QueryHandler, OrchestrationQueryHandler,
    GetPaymentIntentQuery, ListPaymentIntentsQuery,
    GetActiveRoutingPolicyQuery,
};
pub use api::OrchestrationApi;
pub use pipeline::OrchestrationPipeline;
