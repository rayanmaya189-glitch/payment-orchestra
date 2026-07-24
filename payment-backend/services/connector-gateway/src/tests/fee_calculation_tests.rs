//! Fee calculation tests.

use crate::domain;

use super::{sample_fees};

#[test]
fn test_fee_calculation_basic() {
    let fees = sample_fees();
    let amount = domain::Money {
        amount_minor_units: 100000, // 1,000.00 AED
        currency: "AED".into(),
    };

    let fee = fees.calculate_fee(&amount, false, false, 0);
    // fixed_fee (100) + percentage (100000 * 250 / 10000 = 2500) = 2600
    assert_eq!(fee.amount_minor_units, 2600);
}

#[test]
fn test_fee_calculation_with_cross_border() {
    let fees = sample_fees();
    let amount = domain::Money {
        amount_minor_units: 100000,
        currency: "AED".into(),
    };

    let fee = fees.calculate_fee(&amount, true, false, 0);
    // fixed_fee (100) + percentage (2500) + cross_border (50 bps = 500) = 3100
    assert_eq!(fee.amount_minor_units, 3100);
}

#[test]
fn test_fee_calculation_capped() {
    let fees = domain::FeeStructure {
        max_fee_cap: Some(500),
        ..sample_fees()
    };
    let amount = domain::Money {
        amount_minor_units: 1000000, // 10,000 AED
        currency: "AED".into(),
    };

    let fee = fees.calculate_fee(&amount, false, false, 0);
    // Without cap: 100 + 25000 = 25100 → capped at 500
    assert_eq!(fee.amount_minor_units, 500);
}

#[test]
fn test_fee_calculation_floor() {
    let fees = domain::FeeStructure {
        fixed_fee_minor: 0,
        percentage_fee_bps: 10,
        min_fee_floor: Some(100),
        ..sample_fees()
    };
    let amount = domain::Money {
        amount_minor_units: 100, // very small amount
        currency: "AED".into(),
    };

    let fee = fees.calculate_fee(&amount, false, false, 0);
    // Without floor: 0 + 0 = 0 → floored at 100
    assert_eq!(fee.amount_minor_units, 100);
}

#[test]
fn test_fee_calculation_tiered_pricing() {
    let fees = domain::FeeStructure {
        tiered_pricing: Some(vec![
            domain::FeeTier {
                min_volume_minor: 0,
                max_volume_minor: Some(1000000),
                percentage_fee_bps: 300, // 3% for low volume
            },
            domain::FeeTier {
                min_volume_minor: 1000000,
                max_volume_minor: None,
                percentage_fee_bps: 150, // 1.5% for high volume
            },
        ]),
        ..sample_fees()
    };
    let amount = domain::Money {
        amount_minor_units: 100000,
        currency: "AED".into(),
    };

    // Low volume tier: fixed (100) + percentage at 300bps (100000 * 300 / 10000 = 3000) = 3100
    let fee_low = fees.calculate_fee(&amount, false, false, 500000);
    assert_eq!(fee_low.amount_minor_units, 3100);

    // High volume tier: fixed (100) + percentage at 150bps (100000 * 150 / 10000 = 1500) = 1600
    let fee_high = fees.calculate_fee(&amount, false, false, 2000000);
    assert_eq!(fee_high.amount_minor_units, 1600);
}
