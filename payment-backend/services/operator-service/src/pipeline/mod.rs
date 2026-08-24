//! Command processing pipeline for operator-service.
//! Applies authorization, logging, metrics, and other cross-cutting concerns.

use tracing::info;
use uuid::Uuid;

/// Middleware context passed through the command pipeline
#[allow(dead_code)]
pub struct CommandContext {
    pub correlation_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_type: String,
}

impl Default for CommandContext {
    fn default() -> Self {
        Self {
            correlation_id: Uuid::now_v7(),
            actor_id: None,
            actor_type: "system".into(),
        }
    }
}

/// Log entry for audit trail
#[allow(dead_code)]
pub struct AuditEntry {
    pub action: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub details: String,
    pub timestamp_ms: i64,
}

/// Log an audit entry
#[allow(dead_code)]
pub fn log_audit(action: &str, resource_type: &str, resource_id: Uuid, ctx: &CommandContext, details: &str) {
    info!(
        action = action,
        resource_type = resource_type,
        resource_id = %resource_id,
        actor_id = ?ctx.actor_id,
        correlation_id = %ctx.correlation_id,
        details = details,
        "Audit log"
    );
}
