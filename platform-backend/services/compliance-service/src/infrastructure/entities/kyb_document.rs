use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::KybDocument;
use crate::domain::value_objects::KybDocumentType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "kyb_documents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub kyb_case_id: Uuid,
    pub document_type: String,
    pub file_key: String,
    pub file_hash: String,
    pub verified: bool,
    pub uploaded_at: DateTimeWithTimeZone,
    pub verified_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::kyb_case_entity::Entity",
        from = "Column::KybCaseId",
        to = "super::kyb_case_entity::Column::Id"
    )]
    KybCase,
}

impl Related<super::kyb_case_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KybCase.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> KybDocument {
        KybDocument {
            id: self.id,
            kyb_case_id: self.kyb_case_id,
            document_type: KybDocumentType::from_str(&self.document_type),
            file_key: self.file_key.clone(),
            file_hash: self.file_hash.clone(),
            uploaded_at: self.uploaded_at.into(),
            verified: self.verified,
            verified_at: self.verified_at.map(|dt| dt.into()),
        }
    }
}

impl From<KybDocument> for ActiveModel {
    fn from(d: KybDocument) -> Self {
        Self {
            id: sea_orm::Set(d.id),
            kyb_case_id: sea_orm::Set(d.kyb_case_id),
            document_type: sea_orm::Set(d.document_type.as_str().to_string()),
            file_key: sea_orm::Set(d.file_key),
            file_hash: sea_orm::Set(d.file_hash),
            verified: sea_orm::Set(d.verified),
            uploaded_at: sea_orm::Set(d.uploaded_at.into()),
            verified_at: sea_orm::Set(d.verified_at.map(|dt| dt.into())),
        }
    }
}
