//! Event type definitions for invoice-service.

use chrono::{DateTime, Utc};

// ─── Event type string constants ─────────────────────────────────────────────

pub const INVOICE_CREATED: &str = "invoice.created";
pub const INVOICE_SENT: &str = "invoice.sent";
pub const INVOICE_CANCELLED: &str = "invoice.cancelled";
pub const INVOICE_PAID: &str = "invoice.paid";
pub const INVOICE_PARTIALLY_PAID: &str = "invoice.partially_paid";
pub const INVOICE_OVERDUE: &str = "invoice.overdue";
pub const PAYMENT_LINKED: &str = "invoice.payment_linked";
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

impl InvoiceEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::InvoiceCreated(_) => INVOICE_CREATED,
            Self::InvoiceSent(_) => INVOICE_SENT,
            Self::InvoiceCancelled(_) => INVOICE_CANCELLED,
            Self::InvoicePaid(_) => INVOICE_PAID,
            Self::InvoicePartiallyPaid(_) => INVOICE_PARTIALLY_PAID,
            Self::InvoiceOverdue(_) => INVOICE_OVERDUE,
            Self::PaymentLinked(_) => PAYMENT_LINKED,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::InvoiceCreated(e) => e.occurred_at,
            Self::InvoiceSent(e) => e.occurred_at,
            Self::InvoiceCancelled(e) => e.occurred_at,
            Self::InvoicePaid(e) => e.occurred_at,
            Self::InvoicePartiallyPaid(e) => e.occurred_at,
            Self::InvoiceOverdue(e) => e.occurred_at,
            Self::PaymentLinked(e) => e.occurred_at,
        }
    }
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
