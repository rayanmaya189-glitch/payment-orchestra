use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::GatewayProfile;
use crate::domain::value_objects::GatewayProfileStatus;
use shared_types::{CardScheme, CurrencyCode};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gateway_profile")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub status: String,

    pub min_transaction_amount_minor: i64,
    pub max_transaction_amount_minor: i64,
    pub daily_volume_limit_minor: i64,
    pub monthly_volume_limit_minor: i64,
    pub max_refund_amount_minor: i64,

    pub fixed_fee_minor: i64,
    pub percentage_fee_bps: i32,
    pub cross_border_fee_bps: i32,
    pub currency_conversion_fee_bps: i32,

    pub routing_priority: i32,
    pub enabled_card_schemes: String, // JSON array
    pub enabled_currencies: String,   // JSON array
    pub enabled_countries: String,    // JSON array

    pub rate_limit_per_second: u32,
    pub rate_limit_per_day: u32,

    pub success_rate_threshold: f64,
    pub latency_threshold_ms: u32,
    pub auto_disable_on_low_success: bool,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> GatewayProfile {
        let enabled_card_schemes: Vec<CardScheme> =
            serde_json::from_str(&self.enabled_card_schemes).unwrap_or_default();
        let enabled_currencies: Vec<CurrencyCode> =
            serde_json::from_str(&self.enabled_currencies).unwrap_or_default();
        let enabled_countries: Vec<String> =
            serde_json::from_str(&self.enabled_countries).unwrap_or_default();

        GatewayProfile {
            profile_id: self.profile_id,
            operator_id: self.operator_id,
            connector_id: self.connector_id.clone(),
            merchant_acquirer_link_id: self.merchant_acquirer_link_id,
            status: GatewayProfileStatus::from_str(&self.status),
            min_transaction_amount_minor: self.min_transaction_amount_minor,
            max_transaction_amount_minor: self.max_transaction_amount_minor,
            daily_volume_limit_minor: self.daily_volume_limit_minor,
            monthly_volume_limit_minor: self.monthly_volume_limit_minor,
            max_refund_amount_minor: self.max_refund_amount_minor,
            fixed_fee_minor: self.fixed_fee_minor,
            percentage_fee_bps: self.percentage_fee_bps,
            cross_border_fee_bps: self.cross_border_fee_bps,
            currency_conversion_fee_bps: self.currency_conversion_fee_bps,
            routing_priority: self.routing_priority,
            enabled_card_schemes,
            enabled_currencies,
            enabled_countries,
            rate_limit_per_second: self.rate_limit_per_second,
            rate_limit_per_day: self.rate_limit_per_day,
            success_rate_threshold: self.success_rate_threshold,
            latency_threshold_ms: self.latency_threshold_ms,
            auto_disable_on_low_success: self.auto_disable_on_low_success,
            created_at: self.created_at.into(),
            updated_at: self.updated_at.into(),
        }
    }
}

impl From<GatewayProfile> for ActiveModel {
    fn from(p: GatewayProfile) -> Self {
        Self {
            profile_id: sea_orm::Set(p.profile_id),
            operator_id: sea_orm::Set(p.operator_id),
            connector_id: sea_orm::Set(p.connector_id),
            merchant_acquirer_link_id: sea_orm::Set(p.merchant_acquirer_link_id),
            status: sea_orm::Set(p.status.as_str().to_string()),
            min_transaction_amount_minor: sea_orm::Set(p.min_transaction_amount_minor),
            max_transaction_amount_minor: sea_orm::Set(p.max_transaction_amount_minor),
            daily_volume_limit_minor: sea_orm::Set(p.daily_volume_limit_minor),
            monthly_volume_limit_minor: sea_orm::Set(p.monthly_volume_limit_minor),
            max_refund_amount_minor: sea_orm::Set(p.max_refund_amount_minor),
            fixed_fee_minor: sea_orm::Set(p.fixed_fee_minor),
            percentage_fee_bps: sea_orm::Set(p.percentage_fee_bps),
            cross_border_fee_bps: sea_orm::Set(p.cross_border_fee_bps),
            currency_conversion_fee_bps: sea_orm::Set(p.currency_conversion_fee_bps),
            routing_priority: sea_orm::Set(p.routing_priority),
            enabled_card_schemes: sea_orm::Set(serde_json::to_string(&p.enabled_card_schemes).unwrap()),
            enabled_currencies: sea_orm::Set(serde_json::to_string(&p.enabled_currencies).unwrap()),
            enabled_countries: sea_orm::Set(serde_json::to_string(&p.enabled_countries).unwrap()),
            rate_limit_per_second: sea_orm::Set(p.rate_limit_per_second),
            rate_limit_per_day: sea_orm::Set(p.rate_limit_per_day),
            success_rate_threshold: sea_orm::Set(p.success_rate_threshold),
            latency_threshold_ms: sea_orm::Set(p.latency_threshold_ms),
            auto_disable_on_low_success: sea_orm::Set(p.auto_disable_on_low_success),
            created_at: sea_orm::Set(p.created_at.into()),
            updated_at: sea_orm::Set(p.updated_at.into()),
        }
    }
}
