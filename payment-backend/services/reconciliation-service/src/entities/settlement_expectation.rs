//! SettlementExpectation entity — `settlement_expectations` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `settlement_expectations` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "settlement_expectations")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub expectation_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub payment_intent_id: Uuid,
    pub expected_amount_minor: i64,
    pub currency: String,
    pub expected_settlement_date: DateTimeUtc,
    pub status: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
