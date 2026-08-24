//! FeeVariance entity — `fee_variances` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `fee_variances` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fee_variances")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub variance_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub payment_intent_id: Uuid,
    pub expected_fee_minor: i64,
    pub actual_fee_minor: i64,
    pub variance_amount_minor: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
