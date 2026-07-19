use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shared_types::Money;

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub order_reference: String,
    pub line_items: Vec<InvoiceLineItemRequest>,
    pub due_date: String,
    pub recipient_email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceLineItemRequest {
    pub description: String,
    pub amount_minor_units: i64,
    pub quantity: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct RecordPaymentRequest {
    pub payment_intent_id: Uuid,
    pub amount_minor_units: i64,
}

#[derive(Debug, Serialize)]
pub struct InvoiceResponse {
    pub invoice_id: Uuid,
    pub order_reference: String,
    pub status: String,
    pub total_amount: i64,
    pub paid_amount: i64,
    pub currency: String,
    pub due_date: String,
    pub recipient_email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}
