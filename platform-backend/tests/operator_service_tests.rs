//! Integration tests for Operator service — registration, state machine.

#[cfg(test)]
mod operator_tests {
    use uuid::Uuid;
    use shared_types::Money;
    use shared_types::CurrencyCode;

    #[test]
    fn test_money_creation() {
        let money = Money {
            amount_minor_units: 10000,
            currency: CurrencyCode::new("AED").unwrap(),
        };
        assert_eq!(money.amount_minor_units, 10000);
    }

    #[test]
    fn test_money_addition() {
        let a = Money { amount_minor_units: 5000, currency: CurrencyCode::new("AED").unwrap() };
        let b = Money { amount_minor_units: 3000, currency: CurrencyCode::new("AED").unwrap() };
        let result = a.checked_add(&b).unwrap();
        assert_eq!(result.amount_minor_units, 8000);
    }

    #[test]
    fn test_money_addition_overflow() {
        let a = Money { amount_minor_units: i64::MAX, currency: CurrencyCode::new("AED").unwrap() };
        let b = Money { amount_minor_units: 1, currency: CurrencyCode::new("AED").unwrap() };
        assert!(a.checked_add(&b).is_none());
    }

    #[test]
    fn test_payment_status_transitions() {
        use shared_types::PaymentStatus;
        use shared_types::PaymentCommand;

        // Created -> Authorizing
        assert!(PaymentStatus::Created.can_transition(&PaymentCommand::Authorize));

        // Created -> Capturing (not allowed)
        assert!(!PaymentStatus::Created.can_transition(&PaymentCommand::Capture));

        // Authorizing -> Authorized
        assert!(PaymentStatus::Authorizing.can_transition(&PaymentCommand::Authorize));

        // Authorized -> Capturing
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Capture));

        // Authorized -> Voiding
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Void));
    }
}
