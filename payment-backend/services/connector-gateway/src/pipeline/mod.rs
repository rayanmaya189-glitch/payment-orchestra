//! Pipeline stubs for connector-gateway.

use chrono::{DateTime, Utc};

#[allow(dead_code)]
pub struct CommandContext {
    pub trace_id: String,
    pub actor_id: String,
    pub timestamp: DateTime<Utc>,
}

#[allow(dead_code)]
impl Default for CommandContext {
    fn default() -> Self {
        Self {
            trace_id: uuid::Uuid::now_v7().to_string(),
            actor_id: String::new(),
            timestamp: Utc::now(),
        }
    }
}

#[allow(dead_code)]
pub fn log_audit(_action: &str, _context: &CommandContext, _details: &str) {
    // Placeholder for audit logging
}
