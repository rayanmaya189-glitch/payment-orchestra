//! GatewayProfile entity — `gateway_profiles` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gateway_profiles")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub profile_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub connector_id: String,
    #[sea_orm(column_type = "Uuid")]
    pub merchant_acquirer_link_id: Uuid,
    pub status: String,
    pub limits_min_amount_minor: i64,
    pub limits_max_amount_minor: i64,
    pub limits_daily_volume_minor: i64,
    pub limits_monthly_volume_minor: i64,
    pub limits_max_refund_minor: i64,
    pub fee_fixed_minor: i64,
    pub fee_percentage_bps: i32,
    pub fee_cross_border_bps: i32,
    pub fee_currency_conversion_bps: i32,
    pub fee_max_cap: Option<i64>,
    pub fee_min_floor: Option<i64>,
    pub routing_priority: i32,
    pub enabled_card_schemes: Json,
    pub enabled_currencies: Json,
    pub enabled_countries: Json,
    pub rate_limit_per_second: i32,
    pub rate_limit_per_day: i32,
    pub rate_limit_burst: i32,
    pub monitoring_success_rate_alert: f64,
    pub monitoring_success_rate_critical: f64,
    pub monitoring_latency_p99_alert_ms: i32,
    pub monitoring_latency_p99_critical_ms: i32,
    pub monitoring_auto_disable: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
