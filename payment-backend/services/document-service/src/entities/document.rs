//! Document entity — `documents` table.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "documents")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub document_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub uploaded_by: Uuid,
    pub category: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub status: String,
    pub ocr_result: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
