//! SeaORM entity model for the dispute-service's `chargeback_cases` table.
//!
//! Nested complex types (Vec) are stored as JSONB columns.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `chargeback_cases` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chargeback_cases")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub chargeback_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub payment_intent_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub acquirer_link_id: Uuid,
    pub status: String,
    pub reason_code: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub received_at: DateTimeUtc,
    pub representment_deadline: DateTimeUtc,
    pub resolved_at: Option<DateTimeUtc>,
    pub outcome: Option<String>,
    pub resolution_note: Option<String>,
    /// JSONB: serialized Vec<RepresentmentSubmission>
    pub submissions: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
