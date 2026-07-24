//! Connector-gateway API — handler exports for the modular monolith.
//! In a modular monolith, modules communicate via in-process traits (not network gRPC).

pub mod health;
pub mod grpc;
pub mod rate_limit;

pub use crate::commands::{CommandHandler, GatewayCommandHandler};
pub use crate::queries::{QueryHandler, GatewayQueryHandler};
pub use crate::domain::{ConnectorRegistry, GatewayProfile, ConnectorCapabilities};

/// Combined service interface for the API gateway to use.
pub struct ConnectorGatewayApi {
    pub commands: Box<dyn CommandHandler>,
    pub queries: Box<dyn QueryHandler>,
    pub registry: ConnectorRegistry,
}
