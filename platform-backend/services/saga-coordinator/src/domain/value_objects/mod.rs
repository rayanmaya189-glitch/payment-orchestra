#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

/// Saga types per SRS Part 3 §9.2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaType {
    /// Payment lifecycle: create → authorize → capture → complete
    PaymentLifecycle,
    /// Subscription renewal: check → authorize → bill → notify
    SubscriptionRenewal,
    /// Reconciliation resolution: match → verify → resolve
    ReconciliationResolution,
    /// Invoice payment: create → send → pay → confirm
    InvoicePayment,
}

impl SagaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PaymentLifecycle => "payment_lifecycle",
            Self::SubscriptionRenewal => "subscription_renewal",
            Self::ReconciliationResolution => "reconciliation_resolution",
            Self::InvoicePayment => "invoice_payment",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "payment_lifecycle" => Ok(Self::PaymentLifecycle),
            "subscription_renewal" => Ok(Self::SubscriptionRenewal),
            "reconciliation_resolution" => Ok(Self::ReconciliationResolution),
            "invoice_payment" => Ok(Self::InvoicePayment),
            _ => Err("unknown saga type"),
        }
    }
}

/// Saga instance status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaStatus {
    Pending,
    Running,
    Compensating,
    Completed,
    Compensated,
    Failed,
}

impl SagaStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Compensating => "compensating",
            Self::Completed => "completed",
            Self::Compensated => "compensated",
            Self::Failed => "failed",
        }
    }
}

/// Individual step status within a saga.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Compensated,
}

impl SagaStepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Compensated => "compensated",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saga_type_values() {
        assert_eq!(SagaType::PaymentLifecycle.as_str(), "payment_lifecycle");
        assert_eq!(SagaType::SubscriptionRenewal.as_str(), "subscription_renewal");
    }

    #[test]
    fn test_saga_type_from_str() {
        assert_eq!(SagaType::from_str("payment_lifecycle").unwrap(), SagaType::PaymentLifecycle);
        assert!(SagaType::from_str("unknown").is_err());
    }

    #[test]
    fn test_saga_status_values() {
        assert_eq!(SagaStatus::Pending.as_str(), "pending");
        assert_eq!(SagaStatus::Completed.as_str(), "completed");
    }

    #[test]
    fn test_saga_step_status_values() {
        assert_eq!(SagaStepStatus::Pending.as_str(), "pending");
        assert_eq!(SagaStepStatus::Compensated.as_str(), "compensated");
    }
}
