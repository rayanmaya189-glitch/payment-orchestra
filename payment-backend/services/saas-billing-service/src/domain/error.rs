//! Error types for SaaS Billing domain.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaaSbillingError {
    #[error("Plan not found: {0}")]
    PlanNotFound(String),

    #[error("Plan is inactive: {0}")]
    PlanInactive(String),

    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(uuid::Uuid),

    #[error("Subscription already exists for operator: {0}")]
    SubscriptionAlreadyExists(uuid::Uuid),

    #[error("Invalid subscription status transition: {from} -> {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("Cannot cancel subscription during trial")]
    CannotCancelDuringTrial,

    #[error("Usage record not found for period")]
    UsageNotFound,

    #[error("Invoice not found: {0}")]
    InvoiceNotFound(uuid::Uuid),

    #[error("Invoice already paid: {0}")]
    InvoiceAlreadyPaid(uuid::Uuid),

    #[error("Team member limit exceeded: {limit}")]
    TeamMemberLimitExceeded { limit: i32 },

    #[error("Team member not found: {0}")]
    TeamMemberNotFound(uuid::Uuid),

    #[error("Audit log write failed: {0}")]
    AuditLogError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<SaaSbillingError> for platform_error::PlatformError {
    fn from(e: SaaSbillingError) -> Self {
        match e {
            SaaSbillingError::PlanNotFound(ref id) => {
                platform_error::PlatformError::NotFound {
                    resource: "saas_plan",
                    id: Uuid::parse_str(id).unwrap_or_default(),
                }
            }
            SaaSbillingError::PlanInactive(ref slug) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "plan_id".into(),
                        reason: format!("Plan '{}' is inactive", slug),
                    },
                )
            }
            SaaSbillingError::SubscriptionNotFound(id) => {
                platform_error::PlatformError::NotFound {
                    resource: "tenant_subscription",
                    id,
                }
            }
            SaaSbillingError::SubscriptionAlreadyExists(op_id) => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict,
                )
            }
            SaaSbillingError::InvalidStatusTransition { from, to } => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidStateTransition {
                        from_state: from,
                        command: to,
                    },
                )
            }
            SaaSbillingError::CannotCancelDuringTrial => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "status".into(),
                        reason: "Cannot cancel subscription during trial".into(),
                    },
                )
            }
            SaaSbillingError::UsageNotFound => {
                platform_error::PlatformError::NotFound {
                    resource: "tenant_usage",
                    id: uuid::Uuid::nil(),
                }
            }
            SaaSbillingError::InvoiceNotFound(id) => {
                platform_error::PlatformError::NotFound {
                    resource: "saas_invoice",
                    id,
                }
            }
            SaaSbillingError::InvoiceAlreadyPaid(id) => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict,
                )
            }
            SaaSbillingError::TeamMemberLimitExceeded { limit } => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "team_members".into(),
                        reason: format!("Team member limit exceeded: {}", limit),
                    },
                )
            }
            SaaSbillingError::TeamMemberNotFound(id) => {
                platform_error::PlatformError::NotFound {
                    resource: "team_member",
                    id,
                }
            }
            SaaSbillingError::AuditLogError(ref msg) => {
                platform_error::PlatformError::Internal(format!("Audit log error: {}", msg))
            }
            SaaSbillingError::DatabaseError(ref msg) => {
                platform_error::PlatformError::Internal(format!("Database error: {}", msg))
            }
            SaaSbillingError::ValidationError(ref msg) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "unknown".into(),
                        reason: msg.clone(),
                    },
                )
            }
            SaaSbillingError::InternalError(ref msg) => {
                platform_error::PlatformError::Internal(msg.clone())
            }
        }
    }
}
