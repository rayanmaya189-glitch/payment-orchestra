//! PostgreSQL-backed GatewayProfileRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::GatewayProfileRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as ProfileActiveModel,
    Column as ProfileColumn,
    Entity as ProfileEntity,
    Model as ProfileModel,
};

pub struct PostgresGatewayProfileRepository {
    pub db: DatabaseConnection,
}

impl PostgresGatewayProfileRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GatewayProfileRepository for PostgresGatewayProfileRepository {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, String> {
        let result = ProfileEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, profile: &GatewayProfile) -> Result<(), String> {
        let model = domain_to_model(profile);
        let exists = ProfileEntity::find_by_id(profile.profile_id)
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?
            .is_some();

        if exists {
            ProfileEntity::update(ProfileActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            ProfileEntity::insert(ProfileActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String> {
        let profiles = ProfileEntity::find()
            .filter(ProfileColumn::OperatorId.eq(operator_id))
            .filter(ProfileColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        profiles.into_iter().map(|m| model_to_domain(m)).collect()
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, String> {
        let profiles = ProfileEntity::find()
            .filter(ProfileColumn::ConnectorId.eq(connector_id))
            .all(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        profiles.into_iter().map(|m| model_to_domain(m)).collect()
    }

    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, String> {
        let result = ProfileEntity::find()
            .filter(ProfileColumn::MerchantAcquirerLinkId.eq(link_id))
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn check_daily_volume(&self, _profile_id: Uuid) -> Result<Money, String> {
        Ok(Money { amount_minor_units: 0, currency: "AED".into() })
    }

    async fn check_monthly_volume(&self, _profile_id: Uuid) -> Result<Money, String> {
        Ok(Money { amount_minor_units: 0, currency: "AED".into() })
    }

    async fn increment_daily_volume(&self, _profile_id: Uuid, _amount: Money) -> Result<(), String> {
        Ok(())
    }

    async fn get_success_rate(&self, _profile_id: Uuid, _window_hours: u32) -> Result<f64, String> {
        Ok(1.0)
    }

    async fn list_all(&self) -> Result<Vec<GatewayProfile>, String> {
        let profiles = ProfileEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        profiles.into_iter().map(|m| model_to_domain(m)).collect()
    }
}

fn domain_to_model(p: &GatewayProfile) -> ProfileModel {
    let enabled_card_schemes = serde_json::to_value(&p.enabled_card_schemes).unwrap_or_default();
    let enabled_currencies = serde_json::to_value(&p.enabled_currencies).unwrap_or_default();
    let enabled_countries = serde_json::to_value(&p.enabled_countries).unwrap_or_default();

    ProfileModel {
        profile_id: p.profile_id,
        operator_id: p.operator_id,
        connector_id: p.connector_id.clone(),
        merchant_acquirer_link_id: p.merchant_acquirer_link_id,
        status: p.status.as_str().to_string(),
        limits_min_amount_minor: p.limits.min_amount_minor,
        limits_max_amount_minor: p.limits.max_amount_minor,
        limits_daily_volume_minor: p.limits.daily_volume_limit_minor,
        limits_monthly_volume_minor: p.limits.monthly_volume_limit_minor,
        limits_max_refund_minor: p.limits.max_refund_amount_minor,
        fee_fixed_minor: p.fees.fixed_fee_minor,
        fee_percentage_bps: p.fees.percentage_fee_bps,
        fee_cross_border_bps: p.fees.cross_border_fee_bps,
        fee_currency_conversion_bps: p.fees.currency_conversion_fee_bps,
        fee_max_cap: p.fees.max_fee_cap,
        fee_min_floor: p.fees.min_fee_floor,
        routing_priority: p.routing_priority,
        enabled_card_schemes,
        enabled_currencies,
        enabled_countries,
        rate_limit_per_second: p.rate_limits.per_second as i32,
        rate_limit_per_day: p.rate_limits.per_day as i32,
        rate_limit_burst: p.rate_limits.burst_size as i32,
        monitoring_success_rate_alert: p.monitoring.success_rate_alert,
        monitoring_success_rate_critical: p.monitoring.success_rate_critical,
        monitoring_latency_p99_alert_ms: p.monitoring.latency_p99_alert_ms as i32,
        monitoring_latency_p99_critical_ms: p.monitoring.latency_p99_critical_ms as i32,
        monitoring_auto_disable: p.monitoring.auto_disable_on_low_success,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

fn model_to_domain(m: ProfileModel) -> Result<GatewayProfile, String> {
    let status = match m.status.as_str() {
        "active" => ProfileStatus::Active,
        "disabled" => ProfileStatus::Disabled,
        "maintenance" => ProfileStatus::Maintenance,
        _ => return Err(format!("Unknown profile status: {}", m.status)),
    };

    let enabled_card_schemes: Vec<CardScheme> = serde_json::from_value(m.enabled_card_schemes)
        .map_err(|e| format!("Deserialize card_schemes: {}", e))?;
    let enabled_currencies: Vec<String> = serde_json::from_value(m.enabled_currencies)
        .map_err(|e| format!("Deserialize currencies: {}", e))?;
    let enabled_countries: Vec<String> = serde_json::from_value(m.enabled_countries)
        .map_err(|e| format!("Deserialize countries: {}", e))?;

    Ok(GatewayProfile {
        profile_id: m.profile_id,
        operator_id: m.operator_id,
        connector_id: m.connector_id,
        merchant_acquirer_link_id: m.merchant_acquirer_link_id,
        status,
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
            tiered_pricing: None,
        },
        routing_priority: m.routing_priority,
        enabled_card_schemes,
        enabled_currencies,
        enabled_countries,
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
