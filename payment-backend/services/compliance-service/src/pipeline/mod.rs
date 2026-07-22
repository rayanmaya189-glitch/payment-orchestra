//! Command processing pipeline for compliance-service.

#[allow(dead_code)]
pub struct CommandContext {
    pub correlation_id: uuid::Uuid,
    pub actor_id: Option<uuid::Uuid>,
    pub actor_type: String,
}
