//! Payment Link command types — BC-07

use uuid::Uuid;

pub struct CreatePaymentLinkCommand {
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub description: Option<String>,
    pub invoice_id: Option<Uuid>,
    pub expires_in_days: Option<u32>,
}

pub struct ResolvePaymentLinkCommand {
    pub token: String,
    pub payment_intent_id: Uuid,
}

pub struct CancelPaymentLinkCommand {
    pub payment_link_id: Uuid,
    pub reason: Option<String>,
}
