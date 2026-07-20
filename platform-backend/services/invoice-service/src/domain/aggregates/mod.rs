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

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() }
    }

    fn make_line_items() -> Vec<InvoiceLineItem> {
        vec![
            InvoiceLineItem { description: "Item 1".into(), amount_minor_units: 5000, quantity: Some(1) },
            InvoiceLineItem { description: "Item 2".into(), amount_minor_units: 3000, quantity: Some(2) },
        ]
    }

    #[test]
    fn test_new_invoice_is_draft() {
        let inv = Invoice::new(Uuid::now_v7(), "order_123".into(), make_line_items(), Utc::now() + chrono::Duration::days(30), None);
        assert_eq!(inv.status, InvoiceStatus::Draft);
        assert_eq!(inv.total_amount.amount_minor_units, 8000); // 5000 + 3000
        assert_eq!(inv.paid_amount.amount_minor_units, 0);
    }

    #[test]
    fn test_can_send_only_draft() {
        let mut inv = Invoice::new(Uuid::now_v7(), "order_123".into(), make_line_items(), Utc::now() + chrono::Duration::days(30), None);
        assert!(inv.can_send());
        inv.status = InvoiceStatus::Sent;
        assert!(!inv.can_send());
    }

    #[test]
    fn test_can_cancel_draft_or_sent() {
        let mut inv = Invoice::new(Uuid::now_v7(), "order_123".into(), make_line_items(), Utc::now() + chrono::Duration::days(30), None);
        assert!(inv.can_cancel());
        inv.status = InvoiceStatus::Sent;
        assert!(inv.can_cancel());
        inv.status = InvoiceStatus::Paid;
        assert!(!inv.can_cancel());
    }

    #[test]
    fn test_record_payment_partial() {
        let mut inv = Invoice::new(Uuid::now_v7(), "order_123".into(), make_line_items(), Utc::now() + chrono::Duration::days(30), None);
        inv.record_payment(5000);
        assert_eq!(inv.status, InvoiceStatus::PartiallyPaid);
        assert_eq!(inv.paid_amount.amount_minor_units, 5000);
    }

    #[test]
    fn test_record_payment_full() {
        let mut inv = Invoice::new(Uuid::now_v7(), "order_123".into(), make_line_items(), Utc::now() + chrono::Duration::days(30), None);
        inv.record_payment(11000);
        assert_eq!(inv.status, InvoiceStatus::Paid);
        assert_eq!(inv.paid_amount.amount_minor_units, 11000);
    }

    #[test]
    fn test_record_payment_multiple() {
        let mut inv = Invoice::new(Uuid::now_v7(), "order_123".into(), make_line_items(), Utc::now() + chrono::Duration::days(30), None);
        inv.record_payment(5000);
        assert_eq!(inv.status, InvoiceStatus::PartiallyPaid);
        inv.record_payment(6000);
        assert_eq!(inv.status, InvoiceStatus::Paid);
    }

    #[test]
    fn test_invoice_status_values() {
        assert_eq!(InvoiceStatus::Draft.as_str(), "draft");
        assert_eq!(InvoiceStatus::Sent.as_str(), "sent");
        assert_eq!(InvoiceStatus::Paid.as_str(), "paid");
        assert_eq!(InvoiceStatus::PartiallyPaid.as_str(), "partially_paid");
        assert_eq!(InvoiceStatus::Overdue.as_str(), "overdue");
        assert_eq!(InvoiceStatus::Cancelled.as_str(), "cancelled");
    }
}
