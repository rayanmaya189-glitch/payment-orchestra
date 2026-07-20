use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "kyb_case")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub operator_id: Uuid,
    pub status: String,
    pub assigned_compliance_officer: Option<Uuid>,
    pub risk_score: Option<f64>,
    pub decision: Option<String>,
    pub decision_reason: Option<String>,
    pub notes: Option<String>,
    pub submitted_at: DateTimeWithTimeZone,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    pub decided_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::kyb_document_entity::Entity")]
    KybDocument,
}
impl Related<super::kyb_document_entity::Entity> for Entity {
    fn to() -> RelationDef { Relation::KybDocument.def() }
}
impl ActiveModelBehavior for ActiveModel {}
