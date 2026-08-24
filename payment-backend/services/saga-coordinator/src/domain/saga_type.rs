//! SagaType — known saga definitions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaType {
    PaymentLifecycle,
    SubscriptionRenewal,
    ReconciliationResolution,
    InvoicePayment,
}

impl std::fmt::Display for SagaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaymentLifecycle => write!(f, "payment_lifecycle"),
            Self::SubscriptionRenewal => write!(f, "subscription_renewal"),
            Self::ReconciliationResolution => write!(f, "reconciliation_resolution"),
            Self::InvoicePayment => write!(f, "invoice_payment"),
        }
    }
}
