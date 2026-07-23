//! AmlAlert entity — `aml_alerts` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `aml_alerts` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "aml_alerts")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub alert_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub transaction_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub rule_id: String,
    pub details: String,
    pub status: String,
    #[sea_orm(column_type = "Uuid")]
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
