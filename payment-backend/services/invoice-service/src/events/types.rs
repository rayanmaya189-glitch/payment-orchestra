//! Event type definitions for invoice-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvoiceEvent {
    InvoiceCreated(InvoiceCreated),
    InvoiceSent(InvoiceSent),
    InvoiceCancelled(InvoiceCancelled),
    InvoicePaid(InvoicePaid),
    InvoicePartiallyPaid(InvoicePartiallyPaid),
    InvoiceOverdue(InvoiceOverdue),
    PaymentLinked(PaymentLinked),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceCreated {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub order_reference: String,
    pub total_amount_minor: i64,
    pub currency: String,
    pub due_date: DateTime<Utc>,
    pub recipient_email: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSent {
    pub invoice_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceCancelled {
    pub invoice_id: Uuid,
    pub reason: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePaid {
    pub invoice_id: Uuid,
    pub payment_intent_id: Uuid,
    pub paid_amount_minor: i64,
    pub fully_paid: bool,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePartiallyPaid {
    pub invoice_id: Uuid,
    pub payment_intent_id: Uuid,
    pub paid_amount_minor: i64,
    pub remaining_minor: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceOverdue {
    pub invoice_id: Uuid,
    pub due_date: DateTime<Utc>,
    pub days_overdue: u32,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLinked {
    pub invoice_id: Uuid,
    pub payment_intent_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}
