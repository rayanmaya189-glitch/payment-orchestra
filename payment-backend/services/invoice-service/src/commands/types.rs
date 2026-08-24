//! Command type definitions for invoice-service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::*;
use crate::events::InvoiceEvent;

// ─── Command Input Structs ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateInvoice {
    pub operator_id: Uuid,
    pub order_reference: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub currency: String,
    pub due_date: DateTime<Utc>,
    pub recipient_email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SendInvoice {
    pub invoice_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CancelInvoice {
    pub invoice_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinkPaymentToInvoice {
    pub invoice_id: Uuid,
    pub payment_intent_id: Uuid,
    pub amount_minor: i64,
}

#[derive(Debug, Clone)]
pub struct MarkInvoiceOverdue {
    pub invoice_id: Uuid,
}

// ─── Command Result ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResult {
    pub invoice_id: Uuid,
    pub status: InvoiceStatus,
    pub total_amount_minor: i64,
    pub paid_amount_minor: i64,
    pub event: InvoiceEvent,
}
