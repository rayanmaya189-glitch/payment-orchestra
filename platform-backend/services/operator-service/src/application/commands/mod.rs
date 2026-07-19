use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterOperatorCommand {
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyEmailCommand {
    pub operator_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOperatorStatusCommand {
    pub operator_id: Uuid,
    pub new_status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionOperatorCommand {
    pub operator_id: Uuid,
}
