use uuid::Uuid;

use crate::domain::value_objects::PermissionContext;

#[derive(Debug, Clone)]
pub struct ValidatePermissionQuery {
    pub principal_id: Uuid,
    pub resource: String,
    pub action: String,
    pub context: PermissionContext,
}

#[derive(Debug, Clone)]
pub struct GetPrincipalQuery {
    pub principal_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ListApiKeysQuery {
    pub principal_id: Uuid,
}
