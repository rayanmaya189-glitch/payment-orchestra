//! KybCase entity — `kyb_cases` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `kyb_cases` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "kyb_cases")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub kyb_case_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub status: String,
    #[sea_orm(column_type = "Uuid")]
    pub submitted_by: Uuid,
    /// JSONB: serialized Vec<Uuid>
    pub document_ids: Json,
    pub ocr_extracted_fields: Option<String>,
    pub partner_decision: Option<String>,
    pub rejection_reason: Option<String>,
    pub submitted_at: DateTimeUtc,
    pub resolved_at: Option<DateTimeUtc>,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
