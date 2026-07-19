use serde::{Deserialize, Serialize};

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
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Sent => "sent",
            Self::Paid => "paid",
            Self::PartiallyPaid => "partially_paid",
            Self::Overdue => "overdue",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "draft" => Self::Draft,
            "sent" => Self::Sent,
            "paid" => Self::Paid,
            "partially_paid" => Self::PartiallyPaid,
            "overdue" => Self::Overdue,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub amount_minor_units: i64,
    pub quantity: Option<i32>,
}
