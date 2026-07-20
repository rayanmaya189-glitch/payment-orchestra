#![allow(clippy::should_implement_trait)]
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

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "draft" => Ok(Self::Draft),
            "sent" => Ok(Self::Sent),
            "paid" => Ok(Self::Paid),
            "partially_paid" => Ok(Self::PartiallyPaid),
            "overdue" => Ok(Self::Overdue),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err("unknown invoice status"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub amount_minor_units: i64,
    pub quantity: Option<i32>,
}
