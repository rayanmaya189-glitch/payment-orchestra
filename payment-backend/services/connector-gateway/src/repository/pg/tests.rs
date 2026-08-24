//! Tests for GatewayProfile PostgreSQL repository.

use super::gateway_profile::{domain_to_model, model_to_domain};
use crate::domain::*;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_domain_model_roundtrip() {
    let now = Utc::now();
    let profile = GatewayProfile {
        profile_id: Uuid::now_v7(),
        operator_id: Uuid::now_v7(),
        connector_id: "stripe".into(),
        merchant_acquirer_link_id: Uuid::now_v7(),
        status: ProfileStatus::Active,
        limits: TransactionLimits {
            min_amount_minor: 100,
            max_amount_minor: 1_000_000,
            daily_volume_limit_minor: 50_000_000,
            monthly_volume_limit_minor: 1_000_000_000,
            max_refund_amount_minor: 1_000_000,
        },
        fees: FeeStructure {
            fixed_fee_minor: 100,
            percentage_fee_bps: 200,
            cross_border_fee_bps: 50,
            currency_conversion_fee_bps: 100,
            max_fee_cap: Some(5000),
            min_fee_floor: Some(100),
            tiered_pricing: None,
        },
        routing_priority: 1,
        enabled_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
        enabled_currencies: vec!["AED".into(), "USD".into()],
        enabled_countries: vec!["AE".into(), "US".into()],
        rate_limits: RateLimitConfig {
            per_second: 100,
            per_day: 100_000,
            burst_size: 200,
        },
        monitoring: MonitoringThresholds {
            success_rate_alert: 0.95,
            success_rate_critical: 0.90,
            latency_p99_alert_ms: 3000,
            latency_p99_critical_ms: 5000,
            auto_disable_on_low_success: true,
        },
        created_at: now,
        updated_at: now,
    };

    let model = domain_to_model(&profile).unwrap();
    let roundtrip = model_to_domain(model).unwrap();

    assert_eq!(roundtrip.profile_id, profile.profile_id);
    assert_eq!(roundtrip.operator_id, profile.operator_id);
    assert_eq!(roundtrip.connector_id, profile.connector_id);
    assert_eq!(roundtrip.status, profile.status);
    assert_eq!(roundtrip.limits.min_amount_minor, profile.limits.min_amount_minor);
    assert_eq!(roundtrip.fees.fixed_fee_minor, profile.fees.fixed_fee_minor);
    assert_eq!(roundtrip.rate_limits.per_second, profile.rate_limits.per_second);
    assert_eq!(roundtrip.enabled_card_schemes.len(), 2);
    assert_eq!(roundtrip.enabled_currencies.len(), 2);
}
