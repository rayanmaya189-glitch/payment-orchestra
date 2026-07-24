//! Integration tests for connector-gateway.

mod circuit_breaker_tests;
mod fee_calculation_tests;
mod handler_tests;
mod repository_tests;
mod stripe_tests;

pub(crate) mod helpers {
    use uuid::Uuid;

    use crate::domain::{
        FeeStructure, GatewayProfile, MonitoringThresholds,
        ProfileStatus, RateLimitConfig, TransactionLimits,
    };

    pub fn test_profile_id() -> Uuid {
        Uuid::now_v7()
    }

    pub fn test_operator_id() -> Uuid {
        Uuid::now_v7()
    }

    pub fn test_link_id() -> Uuid {
        Uuid::now_v7()
    }

    pub fn sample_limits() -> TransactionLimits {
        TransactionLimits {
            min_amount_minor: 100,
            max_amount_minor: 50000000,
            daily_volume_limit_minor: 5000000000,
            monthly_volume_limit_minor: 50000000000,
            max_refund_amount_minor: 50000000,
        }
    }

    pub fn sample_fees() -> FeeStructure {
        FeeStructure {
            fixed_fee_minor: 100,
            percentage_fee_bps: 250,
            cross_border_fee_bps: 50,
            currency_conversion_fee_bps: 75,
            max_fee_cap: Some(10000),
            min_fee_floor: Some(100),
            tiered_pricing: None,
        }
    }

    pub fn sample_gateway_profile(profile_id: Uuid, operator_id: Uuid, link_id: Uuid, connector_id: &str) -> GatewayProfile {
        GatewayProfile {
            profile_id,
            operator_id,
            connector_id: connector_id.to_string(),
            merchant_acquirer_link_id: link_id,
            status: ProfileStatus::Active,
            limits: sample_limits(),
            fees: sample_fees(),
            routing_priority: 1,
            enabled_card_schemes: vec![crate::domain::CardScheme::Visa, crate::domain::CardScheme::Mastercard],
            enabled_currencies: vec!["AED".into(), "USD".into()],
            enabled_countries: vec!["AE".into()],
            rate_limits: RateLimitConfig {
                per_second: 100,
                per_day: 100000,
                burst_size: 200,
            },
            monitoring: MonitoringThresholds {
                success_rate_alert: 0.95,
                success_rate_critical: 0.90,
                latency_p99_alert_ms: 3000,
                latency_p99_critical_ms: 5000,
                auto_disable_on_low_success: false,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

pub(crate) use helpers::*;
