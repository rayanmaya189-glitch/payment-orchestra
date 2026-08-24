//! PaymentMethodToken entity — `payment_method_tokens` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `payment_method_tokens` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_method_tokens")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub token_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub token_status: String,
    pub payment_method_type: String,
    pub token_ref: String,
    pub created_at: DateTimeUtc,
    pub expires_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
