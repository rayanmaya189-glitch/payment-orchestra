use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub principal_id: Uuid,
    pub principal_type: String,
    pub operator_id: Option<Uuid>,
    pub permissions: Vec<String>,
    pub session_id: Option<Uuid>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
}

impl AuthContext {
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| {
            // Superuser or admin wildcard matches everything
            if p == "*" || p == "admin.*" {
                return true;
            }
            // Resource wildcard: resource:* matches resource:action
            if let Some(resource) = p.strip_suffix(":*") {
                return permission.starts_with(&format!("{}:", resource));
            }
            // Exact match
            p == permission
        })
    }

    pub fn is_admin(&self) -> bool {
        self.permissions.iter().any(|p| p == "admin.*" || p == "*")
    }
}
