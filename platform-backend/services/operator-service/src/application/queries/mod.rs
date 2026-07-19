use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOperatorQuery {
    pub operator_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOperatorsQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorResponse {
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub status: String,
    pub subdomain: String,
    pub email: String,
    pub provisioned_at: Option<String>,
    pub created_at: String,
}
