use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{DocumentStatus, VerificationStatus};

#[derive(Debug, Clone)]
pub struct Document {
    pub document_id: Uuid, pub operator_id: Uuid, pub document_type: String,
    pub status: DocumentStatus, pub filename: String, pub content_type: String,
    pub file_size: i64, pub storage_key: String, pub file_hash: String,
    pub ocr_result: Option<serde_json::Value>, pub metadata: Option<serde_json::Value>,
    pub uploaded_by: String, pub verification_status: VerificationStatus,
    pub verification_notes: Option<String>, pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(operator_id: Uuid, document_type: String, filename: String, content_type: String, file_size: i64, storage_key: String, file_hash: String, uploaded_by: String) -> Self {
        let now = Utc::now();
        Self { document_id: Uuid::now_v7(), operator_id, document_type, status: DocumentStatus::Uploaded,
            filename, content_type, file_size, storage_key, file_hash,
            ocr_result: None, metadata: None, uploaded_by,
            verification_status: VerificationStatus::Pending, verification_notes: None,
            verified_at: None, created_at: now, updated_at: now }
    }

    pub fn verify(&mut self, notes: &str) {
        self.verification_status = VerificationStatus::Verified;
        self.verification_notes = Some(notes.to_string());
        self.verified_at = Some(Utc::now());
        self.status = DocumentStatus::Verified;
        self.updated_at = Utc::now();
    }

    pub fn reject(&mut self, notes: &str) {
        self.verification_status = VerificationStatus::Rejected;
        self.verification_notes = Some(notes.to_string());
        self.status = DocumentStatus::Rejected;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_document() {
        let d = Document::new(Uuid::now_v7(), "trade_license".into(), "license.pdf".into(), "application/pdf".into(), 1024, "key123".into(), "hash123".into(), "user1".into());
        assert_eq!(d.status, DocumentStatus::Uploaded);
        assert_eq!(d.verification_status, VerificationStatus::Pending);
    }

    #[test]
    fn test_verify() {
        let mut d = Document::new(Uuid::now_v7(), "trade_license".into(), "license.pdf".into(), "application/pdf".into(), 1024, "key123".into(), "hash123".into(), "user1".into());
        d.verify("All checks passed");
        assert_eq!(d.verification_status, VerificationStatus::Verified);
        assert!(d.verified_at.is_some());
    }
}
