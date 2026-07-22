//! Invoice-service domain model — Invoice lifecycle management.
//! CRUD + events aggregate: Invoice.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Value Objects ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String,
}

impl Money {
    pub fn zero(currency: &str) -> Self {
        Self { amount_minor_units: 0, currency: currency.to_string() }
    }

    pub fn checked_add(&self, other: &Money) -> Result<Self, InvoiceError> {
        if self.currency != other.currency {
            return Err(InvoiceError::Validation("Currency mismatch".into()));
        }
        let sum = self.amount_minor_units.checked_add(other.amount_minor_units)
            .ok_or_else(|| InvoiceError::Validation("Amount overflow".into()))?;
        Ok(Self { amount_minor_units: sum, currency: self.currency.clone() })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub amount_minor: i64,
    pub quantity: u32,
    pub unit_price_minor: i64,
}

// ─── Invoice Status ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    PartiallyPaid,
    Overdue,
    Cancelled,
}

impl InvoiceStatus {
    pub fn can_transition_to(&self, new_status: &InvoiceStatus) -> Result<(), InvoiceError> {
        match (self, new_status) {
            (Self::Draft, Self::Sent) => Ok(()),
            (Self::Draft, Self::Cancelled) => Ok(()),
            (Self::Sent, Self::Paid) => Ok(()),
            (Self::Sent, Self::PartiallyPaid) => Ok(()),
            (Self::Sent, Self::Overdue) => Ok(()),
            (Self::Sent, Self::Cancelled) => Ok(()),
            (Self::PartiallyPaid, Self::Paid) => Ok(()),
            (Self::Paid, _) => Err(InvoiceError::Validation("Invoice already paid".into())),
            (Self::Cancelled, _) => Err(InvoiceError::Validation("Invoice already cancelled".into())),
            _ => Err(InvoiceError::Validation(format!("Cannot transition from {:?} to {:?}", self, new_status))),
        }
    }
}

// ─── AGG-Invoice ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub order_reference: String,
    pub status: InvoiceStatus,
    pub line_items: Vec<InvoiceLineItem>,
    pub total_amount_minor: i64,
    pub paid_amount_minor: i64,
    pub currency: String,
    pub due_date: DateTime<Utc>,
    pub recipient_email: Option<String>,
    pub payment_intent_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Invoice {
    pub fn new(
        invoice_id: Uuid,
        operator_id: Uuid,
        order_reference: String,
        line_items: Vec<InvoiceLineItem>,
        currency: String,
        due_date: DateTime<Utc>,
        recipient_email: Option<String>,
    ) -> Result<Self, InvoiceError> {
        if line_items.is_empty() {
            return Err(InvoiceError::Validation("Invoice must have at least one line item".into()));
        }

        let total_amount_minor: i64 = line_items.iter()
            .map(|item| item.amount_minor)
            .sum();

        if total_amount_minor <= 0 {
            return Err(InvoiceError::Validation("Invoice total must be positive".into()));
        }

        let now = Utc::now();
        Ok(Self {
            invoice_id,
            operator_id,
            order_reference,
            status: InvoiceStatus::Draft,
            line_items,
            total_amount_minor,
            paid_amount_minor: 0,
            currency,
            due_date,
            recipient_email,
            payment_intent_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn apply_payment(&mut self, amount_minor: i64) -> Result<(), InvoiceError> {
        self.status.can_transition_to(&InvoiceStatus::PartiallyPaid)?;
        self.paid_amount_minor += amount_minor;
        if self.paid_amount_minor >= self.total_amount_minor {
            self.status = InvoiceStatus::Paid;
        } else {
            self.status = InvoiceStatus::PartiallyPaid;
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn mark_overdue(&mut self) -> Result<(), InvoiceError> {
        self.status.can_transition_to(&InvoiceStatus::Overdue)?;
        self.status = InvoiceStatus::Overdue;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), InvoiceError> {
        self.status.can_transition_to(&InvoiceStatus::Cancelled)?;
        self.status = InvoiceStatus::Cancelled;
        self.updated_at = Utc::now();
        Ok(())
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum InvoiceError {
    #[error("Invoice not found: {0}")]
    NotFound(Uuid),

    #[error("Duplicate order invoice: {0}")]
    DuplicateOrderInvoice(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invoice error: {0}")]
    General(String),
}
