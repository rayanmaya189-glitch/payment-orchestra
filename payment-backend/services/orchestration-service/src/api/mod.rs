//! Orchestration-service API — handler exports for the modular monolith.

pub mod grpc;

pub use crate::commands::{CommandHandler, OrchestrationCommandHandler};
pub use crate::queries::{QueryHandler, OrchestrationQueryHandler};
pub use crate::domain::{PaymentIntent, RoutingPolicy, PaymentMethodToken};

/// Combined service interface for the API gateway to use.
pub struct OrchestrationApi {
    pub commands: Box<dyn CommandHandler>,
    pub queries: Box<dyn QueryHandler>,
}
