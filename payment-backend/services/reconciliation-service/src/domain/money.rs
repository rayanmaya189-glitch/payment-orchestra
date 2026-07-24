//! Money value object with checked arithmetic.

use serde::{Deserialize, Serialize};

use super::error::ReconciliationError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String,
}

impl Money {
    pub fn zero(currency: &str) -> Self {
        Self {
            amount_minor_units: 0,
            currency: currency.to_string(),
        }
    }

    pub fn checked_add(&self, other: &Money) -> Result<Self, ReconciliationError> {
        if self.currency != other.currency {
            return Err(ReconciliationError::Validation("Currency mismatch".into()));
        }
        let sum = self
            .amount_minor_units
            .checked_add(other.amount_minor_units)
            .ok_or_else(|| ReconciliationError::Validation("Amount overflow".into()))?;
        Ok(Self {
            amount_minor_units: sum,
            currency: self.currency.clone(),
        })
    }

    pub fn checked_sub(&self, other: &Money) -> Result<Self, ReconciliationError> {
        if self.currency != other.currency {
            return Err(ReconciliationError::Validation("Currency mismatch".into()));
        }
        if self.amount_minor_units < other.amount_minor_units {
            return Err(ReconciliationError::Validation("Insufficient amount".into()));
        }
        Ok(Self {
            amount_minor_units: self.amount_minor_units - other.amount_minor_units,
            currency: self.currency.clone(),
        })
    }
}
