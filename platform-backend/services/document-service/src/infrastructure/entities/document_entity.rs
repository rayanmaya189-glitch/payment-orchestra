use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "document")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub document_type: String,
    pub status: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub storage_key: String,
    pub file_hash: String,
    pub ocr_result: Option<Json>,
    pub metadata: Option<Json>,
    pub uploaded_by: String,
    pub verification_status: String,
    pub verification_notes: Option<String>,
    pub verified_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
