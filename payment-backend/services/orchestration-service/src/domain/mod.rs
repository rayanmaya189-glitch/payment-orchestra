//! Orchestration-service domain model — the core of the payment engine.
//! Event-sourced aggregate: PaymentIntent.
//! CRUD + events aggregates: RoutingPolicy, PaymentMethodToken.
//!
//! Pure Router: This platform NEVER holds funds. It routes transaction *instructions*
//! between merchants, their acquirers/PSPs, and their customers.
//!
//! Each domain concept has its own file within this module.

// Sub-modules — one file per concept
pub mod cost_optimization;
pub mod error;
pub mod idempotency;
pub mod payment_intent;
pub mod payment_method_token;
pub mod payment_status;
pub mod routing_policy;
pub mod smart_routing;
pub mod success_rate;
pub mod value_objects;

// Re-export all types for convenience
pub use error::OrchestrationError;
pub use idempotency::{IdempotencyRecord, IdempotencyResult};
pub use payment_intent::{AttemptStatus, PaymentIntent, RoutingAttempt};
pub use payment_method_token::{PaymentMethodToken, TokenStatus};
pub use payment_status::PaymentStatus;
pub use routing_policy::{
    FailoverConfig, PartialAuthStrategy, PolicyStatus, RotationStrategy,
    RoutingCondition, RoutingPolicy, RoutingRule,
};
pub use cost_optimization::{
    CostOptimizationConfig, CostOptimizer, CostTrackingEntry, GatewayFeeStructure,
    TransactionCost,
};
pub use smart_routing::{
    CircuitBreakerState, GatewayFeatures, RoutingFeatures, RoutingPrediction,
    SmartRoutingConfig, SmartRoutingEngine,
};
pub use success_rate::{GatewaySuccessRate, SlidingWindowStats, SuccessRateTracker};
pub use value_objects::{DeclineReason, FeeBreakdown, Money, PaymentPurpose, SourceType};

// Re-export events for convenience
pub use crate::events::*;
