//! PendingChange entity — `pending_changes` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `pending_changes` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "pending_changes")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub change_id: Uuid,
    pub change_type: String,
    #[sea_orm(column_type = "Uuid")]
    pub maker_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub checker_id: Option<Uuid>,
    pub payload: Vec<u8>,
    pub status: String,
    pub maker_note: Option<String>,
    pub checker_note: Option<String>,
    pub requested_at: DateTimeUtc,
    pub reviewed_at: Option<DateTimeUtc>,
    pub expires_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
