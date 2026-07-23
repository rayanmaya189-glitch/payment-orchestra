//! SeaORM entity model for the subscription-service's `subscriptions` table.
//!
//! Nested complex types (Vec, enums) are stored as JSONB columns.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `subscriptions` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "subscriptions")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub subscription_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub customer_id: Uuid,
    pub plan_id: String,
    pub plan_amount_minor_units: i64,
    pub currency: String,
    pub status: String,
    pub current_period_start: DateTimeUtc,
    pub current_period_end: DateTimeUtc,
    pub billing_interval_days: i64,
    pub payment_method_token_id: Option<Uuid>,
    pub dunning_retry_count: i32,
    pub max_dunning_retries: i32,
    /// JSONB: serialized Vec<BillingCycle>
    pub billing_cycles: Json,
    /// JSONB: serialized Vec<DunningRetry>
    pub dunning_retries: Json,
    pub created_at: DateTimeUtc,
    pub cancelled_at: Option<DateTimeUtc>,
    pub paused_at: Option<DateTimeUtc>,
    pub resumed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
