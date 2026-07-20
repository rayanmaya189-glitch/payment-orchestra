use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "subscription")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub status: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub interval: String,
    pub interval_count: i32,
    pub current_period_start: DateTimeWithTimeZone,
    pub current_period_end: DateTimeWithTimeZone,
    pub trial_period_days: i32,
    pub payment_method_token_id: Option<String>,
    pub failed_payment_intent_id: Option<Uuid>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub canceled_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
