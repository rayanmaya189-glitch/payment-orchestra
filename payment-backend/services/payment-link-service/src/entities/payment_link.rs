//! PaymentLink entity — `payment_links` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_links")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub link_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub description: String,
    pub status: String,
    pub checkout_token: String,
    pub expires_at: DateTimeUtc,
    pub max_uses: i32,
    pub use_count: i32,
    pub success_url: String,
    pub cancel_url: String,
    pub payment_intent_ids: Json,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
