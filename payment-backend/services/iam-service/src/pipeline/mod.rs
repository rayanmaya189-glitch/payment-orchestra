//! Command processing pipeline for iam-service.

use tracing::info;
use uuid::Uuid;

#[allow(dead_code)]
pub struct CommandContext {
    pub correlation_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_type: String,
}

#[allow(dead_code)]
impl Default for CommandContext {
    fn default() -> Self {
        Self {
            correlation_id: Uuid::now_v7(),
            actor_id: None,
            actor_type: "system".into(),
        }
    }
}

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
