use platform_error::ValidationError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: CurrencyCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyCode(pub String);

impl CurrencyCode {
    pub fn new(code: &str) -> Result<Self, ValidationError> {
        if code.len() != 3 || !code.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(ValidationError::InvalidCurrencyCode);
        }
        Ok(Self(code.to_string()))
    }
}

impl Money {
    pub fn minor_unit_precision(&self) -> u32 {
        match self.currency.0.as_str() {
            "BHD" | "KWD" => 3,
            "JPY" => 0,
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

    pub fn checked_add(&self, other: &Money) -> Result<Money, ValidationError> {
        if self.currency != other.currency {
            return Err(ValidationError::CurrencyMismatch);
        }
        let sum = self
            .amount_minor_units
            .checked_add(other.amount_minor_units)
            .ok_or(ValidationError::AmountOverflow)?;
        Ok(Money {
            amount_minor_units: sum,
            currency: self.currency.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_creation_valid() {
        let m = Money {
            amount_minor_units: 10000,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_money_rejects_negative() {
        let m = Money {
            amount_minor_units: -100,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_money_addition_same_currency() {
        let a = Money {
            amount_minor_units: 1000,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        let b = Money {
            amount_minor_units: 2000,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.amount_minor_units, 3000);
    }

    #[test]
    fn test_money_addition_different_currency_fails() {
        let a = Money {
            amount_minor_units: 1000,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        let b = Money {
            amount_minor_units: 2000,
            currency: CurrencyCode::new("USD").unwrap(),
        };
        assert!(a.checked_add(&b).is_err());
    }

    #[test]
    fn test_minor_unit_precision_bhd() {
        let m = Money {
            amount_minor_units: 1000,
            currency: CurrencyCode::new("BHD").unwrap(),
        };
        assert_eq!(m.minor_unit_precision(), 3);
    }

    #[test]
    fn test_minor_unit_precision_jpy() {
        let m = Money {
            amount_minor_units: 1000,
            currency: CurrencyCode::new("JPY").unwrap(),
        };
        assert_eq!(m.minor_unit_precision(), 0);
    }

    #[test]
    fn test_zero_amount_is_card_verification() {
        let m = Money {
            amount_minor_units: 0,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        assert!(m.is_zero());
    }

    #[test]
    fn test_invalid_currency_code() {
        assert!(CurrencyCode::new("AE").is_err());
        assert!(CurrencyCode::new("aE").is_err());
        assert!(CurrencyCode::new("AEAE").is_err());
    }
}
