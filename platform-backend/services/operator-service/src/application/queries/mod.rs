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
