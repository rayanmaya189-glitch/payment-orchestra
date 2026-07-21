use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::value_objects::{DocumentStatus, VerificationStatus};

#[derive(Debug, Clone)]
pub struct Document {
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub document_type: String,
    pub status: DocumentStatus,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub storage_key: String,
    pub file_hash: String,
    pub ocr_result: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub uploaded_by: String,
    pub verification_status: VerificationStatus,
    pub verification_notes: Option<String>,
    pub verified_by: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(
        operator_id: Uuid,
        document_type: String,
        filename: String,
        content_type: String,
        file_size: i64,
        storage_key: String,
        file_hash: String,
        uploaded_by: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            document_id: Uuid::now_v7(),
            operator_id,
            document_type,
            status: DocumentStatus::Uploaded,
            filename,
            content_type,
            file_size,
            storage_key,
            file_hash,
            ocr_result: None,
            metadata: None,
            uploaded_by,
            verification_status: VerificationStatus::Pending,
            verification_notes: None,
            verified_by: None,
            verified_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Verify the document. Only callable from Uploaded status.
    pub fn verify(&mut self, notes: &str, verified_by: &str) -> Result<(), crate::domain::value_objects::DocumentError> {
        if self.status != DocumentStatus::Uploaded {
            return Err(crate::domain::value_objects::DocumentError::InvalidStateTransition {
                from: self.status.as_str().to_string(),
                to: "verified".into(),
            });
        }

        self.verification_status = VerificationStatus::Verified;
        self.verification_notes = Some(notes.to_string());
        self.verified_by = Some(verified_by.to_string());
        self.verified_at = Some(Utc::now());
        self.status = DocumentStatus::Verified;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Reject the document. Only callable from Uploaded status.
    pub fn reject(&mut self, notes: &str, verified_by: &str) -> Result<(), crate::domain::value_objects::DocumentError> {
        if self.status != DocumentStatus::Uploaded {
            return Err(crate::domain::value_objects::DocumentError::InvalidStateTransition {
                from: self.status.as_str().to_string(),
                to: "rejected".into(),
            });
        }

        self.verification_status = VerificationStatus::Rejected;
        self.verification_notes = Some(notes.to_string());
        self.verified_by = Some(verified_by.to_string());
        self.status = DocumentStatus::Rejected;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Store OCR result after processing.
    pub fn set_ocr_result(&mut self, result: serde_json::Value) {
        self.ocr_result = Some(result);
        self.updated_at = Utc::now();
    }

    /// Store additional metadata.
    pub fn set_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = Some(metadata);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc() -> Document {
        Document::new(
            Uuid::now_v7(),
            "trade_license".into(),
            "license.pdf".into(),
            "application/pdf".into(),
            1024,
            "key123".into(),
            "hash123".into(),
            "user1".into(),
        )
    }

    #[test]
    fn test_new_document() {
        let d = make_doc();
        assert_eq!(d.status, DocumentStatus::Uploaded);
        assert_eq!(d.verification_status, VerificationStatus::Pending);
        assert!(d.verified_at.is_none());
    }

    #[test]
    fn test_verify() {
        let mut d = make_doc();
        d.verify("All checks passed", "compliance_officer_1").unwrap();
        assert_eq!(d.verification_status, VerificationStatus::Verified);
        assert_eq!(d.status, DocumentStatus::Verified);
        assert!(d.verified_at.is_some());
        assert_eq!(d.verified_by.as_deref(), Some("compliance_officer_1"));
        assert_eq!(d.verification_notes.as_deref(), Some("All checks passed"));
    }

    #[test]
    fn test_reject() {
        let mut d = make_doc();
        d.reject("Document is blurry", "compliance_officer_2").unwrap();
        assert_eq!(d.verification_status, VerificationStatus::Rejected);
        assert_eq!(d.status, DocumentStatus::Rejected);
        assert_eq!(d.verified_by.as_deref(), Some("compliance_officer_2"));
    }

    #[test]
    fn test_verify_already_verified_fails() {
        let mut d = make_doc();
        d.verify("ok", "officer").unwrap();
        let err = d.verify("again", "officer2").unwrap_err();
        assert!(matches!(err, crate::domain::value_objects::DocumentError::InvalidStateTransition { .. }));
    }

    #[test]
    fn test_reject_already_verified_fails() {
        let mut d = make_doc();
        d.verify("ok", "officer").unwrap();
        let err = d.reject("no", "officer2").unwrap_err();
        assert!(matches!(err, crate::domain::value_objects::DocumentError::InvalidStateTransition { .. }));
    }

    #[test]
    fn test_verify_rejected_fails() {
        let mut d = make_doc();
        d.reject("bad", "officer").unwrap();
        let err = d.verify("ok", "officer2").unwrap_err();
        assert!(matches!(err, crate::domain::value_objects::DocumentError::InvalidStateTransition { .. }));
    }

    #[test]
    fn test_set_ocr_result() {
        let mut d = make_doc();
        let result = serde_json::json!({"text": "Trade License #12345", "confidence": 0.95});
        d.set_ocr_result(result.clone());
        assert_eq!(d.ocr_result, Some(result));
    }

    #[test]
    fn test_set_metadata() {
        let mut d = make_doc();
        let meta = serde_json::json!({"pages": 2, "language": "en"});
        d.set_metadata(meta.clone());
        assert_eq!(d.metadata, Some(meta));
    }

    #[test]
    fn test_document_timestamps() {
        let d = make_doc();
        assert!(d.created_at <= d.updated_at);
        // Both should be recent (within last few seconds)
        let now = Utc::now();
        assert!((now - d.created_at).num_seconds() < 5);
    }
}
