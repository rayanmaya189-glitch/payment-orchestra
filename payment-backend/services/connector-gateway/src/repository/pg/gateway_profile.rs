//! PostgreSQL-backed GatewayProfileRepository using SeaORM CRUD.
//!
//! Converts between the nested domain GatewayProfile model (with TransactionLimits,
//! FeeStructure, RateLimitConfig, MonitoringThresholds sub-structs) and the flat
//! SeaORM entity model (gateway_profiles table with all columns).

use async_trait::async_trait;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::{
    FeeStructure, GatewayProfile, MonitoringThresholds, Money, ProfileStatus,
    RateLimitConfig, TransactionLimits,
};
use crate::entities::gateway_profile::{
    ActiveModel as GatewayProfileActiveModel, Column as GatewayProfileColumn,
    Entity as GatewayProfileEntity, Model as GatewayProfileModel,
};
use super::PostgresConnectorGatewayRepository;
use crate::repository::GatewayProfileRepository;

#[async_trait]
impl GatewayProfileRepository for PostgresConnectorGatewayRepository {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, String> {
        let result = GatewayProfileEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, profile: &GatewayProfile) -> Result<(), String> {
        let model = domain_to_model(profile)?;

        let exists = GatewayProfileEntity::find_by_id(profile.profile_id)
            .one(&self.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .is_some();

        if exists {
            GatewayProfileEntity::update(GatewayProfileActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
        } else {
            GatewayProfileEntity::insert(GatewayProfileActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
        }
        Ok(())
    }

    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String> {
        let models = GatewayProfileEntity::find()
            .filter(GatewayProfileColumn::OperatorId.eq(operator_id))
            .filter(GatewayProfileColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        let mut profiles: Vec<GatewayProfile> = Vec::new();
        for model in models {
            profiles.push(model_to_domain(model)?);
        }
        profiles.sort_by_key(|p| p.routing_priority);
        Ok(profiles)
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, String> {
        let models = GatewayProfileEntity::find()
            .filter(GatewayProfileColumn::ConnectorId.eq(connector_id))
            .all(&self.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, String> {
        let result = GatewayProfileEntity::find()
            .filter(GatewayProfileColumn::MerchantAcquirerLinkId.eq(link_id))
            .one(&self.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn check_daily_volume(&self, profile_id: Uuid) -> Result<Money, String> {
        let vols = self.daily_volumes.read().await;
        let amount = vols.get(&profile_id).copied().unwrap_or(0);
        Ok(Money {
            amount_minor_units: amount,
            currency: "AED".into(),
        })
    }

    async fn check_monthly_volume(&self, profile_id: Uuid) -> Result<Money, String> {
        let vols = self.monthly_volumes.read().await;
        let amount = vols.get(&profile_id).copied().unwrap_or(0);
        Ok(Money {
            amount_minor_units: amount,
            currency: "AED".into(),
        })
    }

    async fn increment_daily_volume(&self, profile_id: Uuid, amount: Money) -> Result<(), String> {
        let mut vols = self.daily_volumes.write().await;
        let current = vols.entry(profile_id).or_insert(0);
        *current = current.saturating_add(amount.amount_minor_units);
        Ok(())
    }

    async fn get_success_rate(&self, profile_id: Uuid, _window_hours: u32) -> Result<f64, String> {
        let counts = self.success_counts.read().await;
        match counts.get(&profile_id) {
            Some((success, total)) if *total > 0 => Ok(*success as f64 / *total as f64),
            _ => Ok(1.0),
        }
    }

    async fn list_all(&self) -> Result<Vec<GatewayProfile>, String> {
        let models = GatewayProfileEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        models.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

/// Convert a domain GatewayProfile to a SeaORM entity model (flat columns).
pub(super) fn domain_to_model(profile: &GatewayProfile) -> Result<GatewayProfileModel, String> {
    let card_schemes_json = serde_json::to_value(&profile.enabled_card_schemes)
        .map_err(|e| format!("Serialize card schemes: {}", e))?;
    let currencies_json = serde_json::to_value(&profile.enabled_currencies)
        .map_err(|e| format!("Serialize currencies: {}", e))?;
    let countries_json = serde_json::to_value(&profile.enabled_countries)
        .map_err(|e| format!("Serialize countries: {}", e))?;

    Ok(GatewayProfileModel {
        profile_id: profile.profile_id,
        operator_id: profile.operator_id,
        connector_id: profile.connector_id.clone(),
        merchant_acquirer_link_id: profile.merchant_acquirer_link_id,
        status: profile.status.as_str().to_string(),
        limits_min_amount_minor: profile.limits.min_amount_minor,
        limits_max_amount_minor: profile.limits.max_amount_minor,
        limits_daily_volume_minor: profile.limits.daily_volume_limit_minor,
        limits_monthly_volume_minor: profile.limits.monthly_volume_limit_minor,
        limits_max_refund_minor: profile.limits.max_refund_amount_minor,
        fee_fixed_minor: profile.fees.fixed_fee_minor,
        fee_percentage_bps: profile.fees.percentage_fee_bps,
        fee_cross_border_bps: profile.fees.cross_border_fee_bps,
        fee_currency_conversion_bps: profile.fees.currency_conversion_fee_bps,
        fee_max_cap: profile.fees.max_fee_cap,
        fee_min_floor: profile.fees.min_fee_floor,
        routing_priority: profile.routing_priority,
        enabled_card_schemes: card_schemes_json,
        enabled_currencies: currencies_json,
        enabled_countries: countries_json,
        rate_limit_per_second: profile.rate_limits.per_second as i32,
        rate_limit_per_day: profile.rate_limits.per_day as i32,
        rate_limit_burst: profile.rate_limits.burst_size as i32,
        monitoring_success_rate_alert: profile.monitoring.success_rate_alert,
        monitoring_success_rate_critical: profile.monitoring.success_rate_critical,
        monitoring_latency_p99_alert_ms: profile.monitoring.latency_p99_alert_ms as i32,
        monitoring_latency_p99_critical_ms: profile.monitoring.latency_p99_critical_ms as i32,
        monitoring_auto_disable: profile.monitoring.auto_disable_on_low_success,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
    })
}

/// Convert a SeaORM entity model back to the domain GatewayProfile.
pub(super) fn model_to_domain(m: GatewayProfileModel) -> Result<GatewayProfile, String> {
    let card_schemes: Vec<crate::domain::CardScheme> =
        serde_json::from_value(m.enabled_card_schemes)
            .unwrap_or_default();
    let currencies: Vec<String> =
        serde_json::from_value(m.enabled_currencies).unwrap_or_default();
    let countries: Vec<String> =
        serde_json::from_value(m.enabled_countries).unwrap_or_default();

    Ok(GatewayProfile {
        profile_id: m.profile_id,
        operator_id: m.operator_id,
        connector_id: m.connector_id,
        merchant_acquirer_link_id: m.merchant_acquirer_link_id,
        status: match m.status.as_str() {
            "active" => ProfileStatus::Active,
            "disabled" => ProfileStatus::Disabled,
            "maintenance" => ProfileStatus::Maintenance,
            _ => ProfileStatus::Active,
        },
        limits: TransactionLimits {
            min_amount_minor: m.limits_min_amount_minor,
            max_amount_minor: m.limits_max_amount_minor,
            daily_volume_limit_minor: m.limits_daily_volume_minor,
            monthly_volume_limit_minor: m.limits_monthly_volume_minor,
            max_refund_amount_minor: m.limits_max_refund_minor,
        },
        fees: FeeStructure {
            fixed_fee_minor: m.fee_fixed_minor,
            percentage_fee_bps: m.fee_percentage_bps,
            cross_border_fee_bps: m.fee_cross_border_bps,
            currency_conversion_fee_bps: m.fee_currency_conversion_bps,
            max_fee_cap: m.fee_max_cap,
            min_fee_floor: m.fee_min_floor,
            tiered_pricing: None, // Not stored in flat entity
        },
        routing_priority: m.routing_priority,
        enabled_card_schemes: card_schemes,
        enabled_currencies: currencies,
        enabled_countries: countries,
        rate_limits: RateLimitConfig {
            per_second: m.rate_limit_per_second as u32,
            per_day: m.rate_limit_per_day as u32,
            burst_size: m.rate_limit_burst as u32,
        },
        monitoring: MonitoringThresholds {
            success_rate_alert: m.monitoring_success_rate_alert,
            success_rate_critical: m.monitoring_success_rate_critical,
            latency_p99_alert_ms: m.monitoring_latency_p99_alert_ms as u32,
            latency_p99_critical_ms: m.monitoring_latency_p99_critical_ms as u32,
            auto_disable_on_low_success: m.monitoring_auto_disable,
        },
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

