//! gRPC service implementation for orchestration-service.
//! Translates between protobuf types and domain types for payment intent lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::CommandHandler;
use crate::domain::OrchestrationError;

use platform_proto::orchestration::orchestration_service_server::OrchestrationService;
use platform_proto::orchestration::*;

pub mod payment;
pub mod routing;

pub struct OrchestrationGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> OrchestrationGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub(crate) fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

pub(crate) fn orchestration_error_to_status(e: OrchestrationError) -> Status {
    match e {
        OrchestrationError::NotFound(id) => Status::not_found(format!("Not found: {}", id)),
        OrchestrationError::InvalidStateTransition(msg) => Status::failed_precondition(msg),
        OrchestrationError::Validation(msg) => Status::invalid_argument(msg),
        OrchestrationError::InvariantViolation(msg) => Status::internal(msg),
        OrchestrationError::ConcurrencyConflict { .. } => Status::aborted("Concurrency conflict"),
        OrchestrationError::IdempotencyConflict(key) => {
            Status::already_exists(format!("Idempotency conflict: {}", key))
        }
        OrchestrationError::NoEligibleRoute => Status::unavailable("No eligible route"),
        OrchestrationError::RoutingPolicyNotFound => Status::not_found("Routing policy not found"),
        OrchestrationError::AllAcquirersDeclined => Status::unavailable("All acquirers declined"),
        OrchestrationError::PaymentMethodTokenInvalid => Status::invalid_argument("Payment method token invalid"),
        OrchestrationError::DatabaseError(msg) => Status::internal(format!("Database error: {}", msg)),
    }
}

impl From<OrchestrationError> for Status {
    fn from(e: OrchestrationError) -> Self {
        orchestration_error_to_status(e)
    }
}
