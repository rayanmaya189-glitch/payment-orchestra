//! SaaS Invoice domain model.

use chrono::{Date, DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::SaaSbillingError;

/// An invoice for a merchant's SaaS subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasInvoice {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub subscription_id: Uuid,
    pub invoice_number: String,
    pub status: InvoiceStatus,
    pub subtotal_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub currency: String,
    pub period_start: Date<Utc>,
    pub period_end: Date<Utc>,
    pub due_date: Date<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub line_items: Vec<InvoiceLineItem>,
    pub stripe_invoice_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invoice status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Paid => "paid",
            Self::Void => "void",
            Self::Uncollectible => "uncollectible",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "open" => Some(Self::Open),
            "paid" => Some(Self::Paid),
            "void" => Some(Self::Void),
            "uncollectible" => Some(Self::Uncollectible),
            _ => None,
        }
    }
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A line item on an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub amount_minor: i64,
    pub quantity: i32,
    pub unit_price_minor: i64,
}

impl SaasInvoice {
    /// Create a new draft invoice.
    pub fn new(
        operator_id: Uuid,
        subscription_id: Uuid,
        invoice_number: String,
        period_start: Date<Utc>,
        period_end: Date<Utc>,
        line_items: Vec<InvoiceLineItem>,
    ) -> Self {
        let now = Utc::now();
        let subtotal: i64 = line_items.iter().map(|item| item.amount_minor).sum();

        Self {
            invoice_id: Uuid::now_v7(),
            operator_id,
            subscription_id,
            invoice_number,
            status: InvoiceStatus::Draft,
            subtotal_minor: subtotal,
            tax_minor: 0,
            total_minor: subtotal,
            currency: "USD".into(),
            period_start,
            period_end,
            due_date: period_end + chrono::Duration::days(7),
            paid_at: None,
            line_items,
            stripe_invoice_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Finalize the invoice (convert from draft to open).
    pub fn finalize(&mut self) -> Result<(), SaaSbillingError> {
        if self.status != InvoiceStatus::Draft {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "open".into(),
            });
        }
        self.status = InvoiceStatus::Open;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark the invoice as paid.
    pub fn mark_paid(&mut self) -> Result<(), SaaSbillingError> {
        if self.status != InvoiceStatus::Open {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "paid".into(),
            });
        }
        self.status = InvoiceStatus::Paid;
        self.paid_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Void the invoice.
    pub fn void(&mut self) -> Result<(), SaaSbillingError> {
        if self.status == InvoiceStatus::Paid {
            return Err(SaaSbillingError::InvoiceAlreadyPaid(self.invoice_id));
        }
        if self.status == InvoiceStatus::Void {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "void".into(),
            });
        }
        self.status = InvoiceStatus::Void;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if the invoice is overdue.
    pub fn is_overdue(&self) -> bool {
        self.status == InvoiceStatus::Open && Utc::now().date_naive() > self.due_date
    }

    /// Add a line item to the invoice.
    pub fn add_line_item(&mut self, item: InvoiceLineItem) {
        self.subtotal_minor += item.amount_minor;
        self.total_minor = self.subtotal_minor + self.tax_minor;
        self.line_items.push(item);
        self.updated_at = Utc::now();
    }
}

impl std::fmt::Display for SaasInvoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invoice {} ({}) - {} {}",
            self.invoice_number, self.status, self.total_minor, self.currency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_line_item() -> InvoiceLineItem {
        InvoiceLineItem {
            description: "Monthly subscription".into(),
            amount_minor: 9900,
            quantity: 1,
            unit_price_minor: 9900,
        }
    }

    fn test_invoice() -> SaasInvoice {
        SaasInvoice::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "INV-2024-001".into(),
            chrono::Utc::now().date_naive(),
            chrono::Utc::now().date_naive() + chrono::Duration::days(30),
            vec![test_line_item()],
        )
    }

    #[test]
    fn test_invoice_new() {
        let invoice = test_invoice();
        assert_eq!(invoice.status, InvoiceStatus::Draft);
        assert_eq!(invoice.subtotal_minor, 9900);
        assert_eq!(invoice.total_minor, 9900);
        assert!(invoice.paid_at.is_none());
    }

    #[test]
    fn test_invoice_finalize() {
        let mut invoice = test_invoice();
        assert!(invoice.finalize().is_ok());
        assert_eq!(invoice.status, InvoiceStatus::Open);
    }

    #[test]
    fn test_invoice_finalize_non_draft_fails() {
        let mut invoice = test_invoice();
        invoice.status = InvoiceStatus::Open;
        assert!(invoice.finalize().is_err());
    }

    #[test]
    fn test_invoice_mark_paid() {
        let mut invoice = test_invoice();
        invoice.status = InvoiceStatus::Open;
        assert!(invoice.mark_paid().is_ok());
        assert_eq!(invoice.status, InvoiceStatus::Paid);
        assert!(invoice.paid_at.is_some());
    }

    #[test]
    fn test_invoice_mark_paid_non_open_fails() {
        let mut invoice = test_invoice();
        assert!(invoice.mark_paid().is_err());
    }

    #[test]
    fn test_invoice_void() {
        let mut invoice = test_invoice();
        assert!(invoice.void().is_ok());
        assert_eq!(invoice.status, InvoiceStatus::Void);
    }

    #[test]
    fn test_invoice_void_paid_fails() {
        let mut invoice = test_invoice();
        invoice.status = InvoiceStatus::Paid;
        assert!(invoice.void().is_err());
    }

    #[test]
    fn test_invoice_is_overdue() {
        let mut invoice = test_invoice();
        invoice.status = InvoiceStatus::Open;
        invoice.due_date = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
        assert!(invoice.is_overdue());

        invoice.due_date = chrono::Utc::now().date_naive() + chrono::Duration::days(7);
        assert!(!invoice.is_overdue());
    }

    #[test]
    fn test_invoice_add_line_item() {
        let mut invoice = test_invoice();
        let item = InvoiceLineItem {
            description: "Overage charges".into(),
            amount_minor: 1500,
            quantity: 1,
            unit_price_minor: 1500,
        };
        invoice.add_line_item(item);
        assert_eq!(invoice.subtotal_minor, 11400);
        assert_eq!(invoice.line_items.len(), 2);
    }

    #[test]
    fn test_invoice_display() {
        let invoice = test_invoice();
        assert_eq!(format!("{}", invoice), "Invoice INV-2024-001 (draft) - 9900 USD");
    }
}
