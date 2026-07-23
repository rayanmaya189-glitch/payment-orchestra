//! LedgerEntry entity — `ledger_entries` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `ledger_entries` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ledger_entries")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub entry_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub transaction_id: Uuid,
    pub entry_type: String,
    pub amount_minor: i64,
    pub currency: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
