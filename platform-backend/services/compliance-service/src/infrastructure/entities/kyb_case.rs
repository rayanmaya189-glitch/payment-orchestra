use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::KybCase;
use crate::domain::value_objects::KybCaseStatus;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "kyb_cases")]
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
    fn to() -> RelationDef {
        Relation::KybDocument.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self, documents: Vec<crate::domain::aggregates::KybDocument>) -> KybCase {
        KybCase {
            id: self.id,
            operator_id: self.operator_id,
            status: KybCaseStatus::from_str(&self.status)
                .expect("DB contains invalid KYB case status"),
            assigned_compliance_officer: self.assigned_compliance_officer,
            documents,
            risk_score: self.risk_score,
            decision: self.decision.as_ref().map(|d| crate::domain::aggregates::KybDecision {
                decision: KybCaseStatus::from_str(d)
                    .expect("DB contains invalid KYB decision status"),
                reason: self.decision_reason.clone().unwrap_or_default(),
                decided_by: self.assigned_compliance_officer.unwrap_or_default(),
                decided_at: self.decided_at.map(|dt| dt.into()).unwrap_or_default(),
            }),
            notes: self.notes.clone(),
            submitted_at: self.submitted_at.into(),
            reviewed_at: self.reviewed_at.map(|dt| dt.into()),
            decided_at: self.decided_at.map(|dt| dt.into()),
            created_at: self.created_at.into(),
        }
    }
}

impl From<KybCase> for ActiveModel {
    fn from(c: KybCase) -> Self {
        Self {
            id: sea_orm::Set(c.id),
            operator_id: sea_orm::Set(c.operator_id),
            status: sea_orm::Set(c.status.as_str().to_string()),
            assigned_compliance_officer: sea_orm::Set(c.assigned_compliance_officer),
            risk_score: sea_orm::Set(c.risk_score),
            decision: sea_orm::Set(c.decision.as_ref().map(|d| d.decision.as_str().to_string())),
            decision_reason: sea_orm::Set(c.decision.as_ref().map(|d| d.reason.clone())),
            notes: sea_orm::Set(c.notes),
            submitted_at: sea_orm::Set(c.submitted_at.into()),
            reviewed_at: sea_orm::Set(c.reviewed_at.map(|dt| dt.into())),
            decided_at: sea_orm::Set(c.decided_at.map(|dt| dt.into())),
            created_at: sea_orm::Set(c.created_at.into()),
        }
    }
}
