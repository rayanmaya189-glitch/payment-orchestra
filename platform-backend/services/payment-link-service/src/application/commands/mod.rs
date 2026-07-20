use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreatePaymentLinkCommand {
    pub operator_id: Uuid, pub description: String, pub merchant_name: String,
    pub amount_minor_units: i64, pub currency: String,
    pub max_uses: Option<i32>, pub expires_in_hours: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UsePaymentLinkCommand { pub public_token: String }
