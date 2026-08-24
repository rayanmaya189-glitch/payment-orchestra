use serde::{Deserialize, Serialize};
use std::fmt;
use platform_error::{ValidationError, PlatformError};

/// ISO 4217 currency code - exactly 3 uppercase ASCII letters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct CurrencyCode(pub String);

impl CurrencyCode {
    pub fn new(code: &str) -> Result<Self, ValidationError> {
        if code.len() != 3 || !code.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(ValidationError::InvalidCurrencyCode);
        }
        Ok(Self(code.to_string()))
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Money value object - always integer minor units, NEVER floats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: CurrencyCode,
}

impl Money {
    pub fn new(amount_minor_units: i64, currency: CurrencyCode) -> Self {
        Self { amount_minor_units, currency }
    }

    pub fn zero(currency: CurrencyCode) -> Self {
        Self { amount_minor_units: 0, currency }
    }

    /// Minor unit precision per ISO 4217
    pub fn minor_unit_precision(&self) -> u32 {
        match self.currency.0.as_str() {
            "BHD" | "KWD" | "OMR" => 3,
            "JPY" | "KRW" | "VND" => 0,
            _ => 2,
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.amount_minor_units < 0 {
            return Err(ValidationError::NegativeAmount);
        }
        Ok(())
    }

    pub fn is_zero(&self) -> bool {
        self.amount_minor_units == 0
    }

    pub fn checked_add(&self, other: &Money) -> Result<Money, PlatformError> {
        if self.currency != other.currency {
            return Err(ValidationError::CurrencyMismatch)?;
        }
        let sum = self.amount_minor_units
            .checked_add(other.amount_minor_units)
            .ok_or(ValidationError::AmountOverflow)?;
        Ok(Money { amount_minor_units: sum, currency: self.currency.clone() })
    }

    pub fn checked_sub(&self, other: &Money) -> Result<Money, PlatformError> {
        if self.currency != other.currency {
            return Err(ValidationError::CurrencyMismatch)?;
        }
        if self.amount_minor_units < other.amount_minor_units {
            return Err(ValidationError::NegativeAmount)?;
        }
        let diff = self.amount_minor_units - other.amount_minor_units;
        Ok(Money { amount_minor_units: diff, currency: self.currency.clone() })
    }

    pub fn checked_mul(&self, multiplier: i64) -> Result<Money, PlatformError> {
        let product = self.amount_minor_units
            .checked_mul(multiplier)
            .ok_or(ValidationError::AmountOverflow)?;
        Ok(Money { amount_minor_units: product, currency: self.currency.clone() })
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let precision = self.minor_unit_precision() as usize;
        let major = self.amount_minor_units / 10i64.pow(precision as u32);
        let minor = (self.amount_minor_units % 10i64.pow(precision as u32)).abs();
        write!(f, "{}.{:0width$} {}", major, minor, self.currency, width = precision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_creation_valid() {
        let m = Money::new(10000, CurrencyCode::new("AED").unwrap());
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_money_rejects_negative() {
        let m = Money::new(-100, CurrencyCode::new("AED").unwrap());
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_money_addition_same_currency() {
        let a = Money::new(1000, CurrencyCode::new("AED").unwrap());
        let b = Money::new(2000, CurrencyCode::new("AED").unwrap());
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.amount_minor_units, 3000);
    }

    #[test]
    fn test_money_addition_different_currency_fails() {
        let a = Money::new(1000, CurrencyCode::new("AED").unwrap());
        let b = Money::new(2000, CurrencyCode::new("USD").unwrap());
        assert!(a.checked_add(&b).is_err());
    }

    #[test]
    fn test_minor_unit_precision_bhd() {
        let m = Money::new(1000, CurrencyCode::new("BHD").unwrap());
        assert_eq!(m.minor_unit_precision(), 3);
    }

    #[test]
    fn test_zero_amount() {
        let m = Money::zero(CurrencyCode::new("AED").unwrap());
        assert!(m.is_zero());
    }

    #[test]
    fn test_display() {
        let m = Money::new(12345, CurrencyCode::new("AED").unwrap());
        assert_eq!(m.to_string(), "123.45 AED");
    }

    #[test]
    fn test_subtraction() {
        let a = Money::new(5000, CurrencyCode::new("AED").unwrap());
        let b = Money::new(3000, CurrencyCode::new("AED").unwrap());
        let diff = a.checked_sub(&b).unwrap();
        assert_eq!(diff.amount_minor_units, 2000);
    }

    #[test]
    fn test_subtraction_negative_fails() {
        let a = Money::new(1000, CurrencyCode::new("AED").unwrap());
        let b = Money::new(3000, CurrencyCode::new("AED").unwrap());
        assert!(a.checked_sub(&b).is_err());
    }
}
