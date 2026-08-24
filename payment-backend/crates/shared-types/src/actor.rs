use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActorReference {
    pub actor_type: ActorType,
    pub actor_id: Option<Uuid>,
}

impl ActorReference {
    pub fn system() -> Self {
        Self { actor_type: ActorType::System, actor_id: None }
    }

    pub fn user(id: Uuid) -> Self {
        Self { actor_type: ActorType::User, actor_id: Some(id) }
    }

    pub fn api_key(id: Uuid) -> Self {
        Self { actor_type: ActorType::ApiKey, actor_id: Some(id) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActorType {
    User,
    ApiKey,
    System,
    Scheduler,
    Webhook,
}
