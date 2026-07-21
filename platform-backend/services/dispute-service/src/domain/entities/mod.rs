use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A piece of evidence attached to a dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_id: Uuid,
    pub dispute_id: Uuid,
    pub evidence_type: EvidenceType,
    pub description: String,
    pub file_uri: Option<String>,
    pub submitted_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    Receipt,
    ShippingProof,
    Communication,
    PolicyDocument,
    ChargebackNotification,
    Other,
}

impl EvidenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Receipt => "receipt",
            Self::ShippingProof => "shipping_proof",
            Self::Communication => "communication",
            Self::PolicyDocument => "policy_document",
            Self::ChargebackNotification => "chargeback_notification",
            Self::Other => "other",
        }
    }
}

impl Evidence {
    pub fn new(dispute_id: Uuid, evidence_type: EvidenceType, description: String, file_uri: Option<String>, submitted_by: Uuid) -> Self {
        Self {
            evidence_id: Uuid::now_v7(),
            dispute_id,
            evidence_type,
            description,
            file_uri,
            submitted_by,
            created_at: Utc::now(),
        }
    }
}
