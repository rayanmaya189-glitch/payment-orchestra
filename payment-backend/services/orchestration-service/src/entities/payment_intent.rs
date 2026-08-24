//! PaymentIntent entity — `payment_intents` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `payment_intents` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_intents")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub payment_intent_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub status: String,
    pub payment_method_type: String,
    pub captured_amount_minor: i64,
    pub refunded_amount_minor: i64,
    /// JSONB: serialized Vec<RoutingAttempt>
    pub routing_attempts: Json,
    pub metadata_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
