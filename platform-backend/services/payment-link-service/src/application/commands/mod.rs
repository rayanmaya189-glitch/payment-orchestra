use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreatePaymentLinkCommand {
    pub operator_id: Uuid,
    pub description: String,
    pub merchant_name: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub max_uses: Option<i32>,
    pub expires_in_hours: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct UsePaymentLinkCommand {
    pub public_token: String,
}

#[derive(Debug, Clone)]
pub struct DeactivatePaymentLinkCommand {
    pub link_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct UpdatePaymentLinkMetadataCommand {
    pub link_id: Uuid,
    pub metadata: serde_json::Value,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct ListPaymentLinksCommand {
    pub operator_id: Uuid,
    pub status: Option<String>,
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub principal_id: Uuid,
    pub role: String,
}
