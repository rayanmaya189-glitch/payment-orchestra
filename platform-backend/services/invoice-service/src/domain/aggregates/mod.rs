use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{InvoiceLineItem, InvoiceStatus};
use shared_types::Money;

#[derive(Debug, Clone)]
pub struct Invoice {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub order_reference: String,
    pub status: InvoiceStatus,
    pub total_amount: Money,
    pub paid_amount: Money,
    pub line_items: Vec<InvoiceLineItem>,
    pub due_date: DateTime<Utc>,
    pub recipient_email: Option<String>,
    pub payment_intent_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Invoice {
    pub fn new(
        operator_id: Uuid,
        order_reference: String,
        line_items: Vec<InvoiceLineItem>,
        due_date: DateTime<Utc>,
        recipient_email: Option<String>,
    ) -> Self {
        let total = line_items.iter().map(|item| item.amount_minor_units).sum::<i64>();
        let currency = shared_types::CurrencyCode::new("AED").unwrap();

        Self {
            invoice_id: Uuid::now_v7(),
            operator_id,
            order_reference,
            status: InvoiceStatus::Draft,
            total_amount: Money {
                amount_minor_units: total,
                currency: currency.clone(),
            },
            paid_amount: Money {
                amount_minor_units: 0,
                currency,
            },
            line_items,
            due_date,
            recipient_email,
            payment_intent_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn can_send(&self) -> bool {
        self.status == InvoiceStatus::Draft
    }

    pub fn can_cancel(&self) -> bool {
        matches!(self.status, InvoiceStatus::Draft | InvoiceStatus::Sent)
    }

    pub fn record_payment(&mut self, amount: i64) {
        self.paid_amount.amount_minor_units += amount;
        if self.paid_amount.amount_minor_units >= self.total_amount.amount_minor_units {
            self.status = InvoiceStatus::Paid;
        } else {
            self.status = InvoiceStatus::PartiallyPaid;
        }
        self.updated_at = Utc::now();
    }
}
