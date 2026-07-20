use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "kyb_document")]
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
    fn to() -> RelationDef { Relation::KybCase.def() }
}
impl ActiveModelBehavior for ActiveModel {}
