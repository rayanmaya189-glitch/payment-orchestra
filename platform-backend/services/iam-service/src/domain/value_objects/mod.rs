#![allow(clippy::should_implement_trait)]
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub resource: String,
    pub action: String,
    pub conditions: Vec<AccessCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessCondition {
    AmountBelow(i64),
    AmountAbove(i64),
    AcquirerLinkIn(Vec<Uuid>),
    OwnerOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    pub amount: Option<i64>,
    pub acquirer_link_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResult {
    pub allowed: bool,
    pub requires_maker_checker: bool,
}
