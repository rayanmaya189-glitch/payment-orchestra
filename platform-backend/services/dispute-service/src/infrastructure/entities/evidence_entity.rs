use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "dispute_evidence")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub evidence_id: Uuid,
    pub dispute_id: Uuid,
    pub evidence_type: String,
    pub description: String,
    pub file_uri: Option<String>,
    pub submitted_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::dispute_entity::Entity",
        from = "Column::DisputeId",
        to = "super::dispute_entity::Column::DisputeId"
    )]
    Dispute,
}

impl Related<super::dispute_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Dispute.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for crate::domain::entities::Evidence {
    fn from(m: Model) -> Self {
        use crate::domain::entities::EvidenceType;
        let evidence_type = match m.evidence_type.as_str() {
            "receipt" => EvidenceType::Receipt,
            "shipping_proof" => EvidenceType::ShippingProof,
            "communication" => EvidenceType::Communication,
            "policy_document" => EvidenceType::PolicyDocument,
            "chargeback_notification" => EvidenceType::ChargebackNotification,
            _ => EvidenceType::Other,
        };
        Self {
            evidence_id: m.evidence_id,
            dispute_id: m.dispute_id,
            evidence_type,
            description: m.description,
            file_uri: m.file_uri,
            submitted_by: m.submitted_by,
            created_at: m.created_at.into(),
        }
    }
}
