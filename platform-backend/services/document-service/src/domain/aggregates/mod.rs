use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{DocumentStatus, DocumentType};

/// Document aggregate — represents a stored document (KYB evidence, reports, etc).
#[derive(Debug, Clone)]
pub struct Document {
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub document_type: DocumentType,
    pub filename: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub storage_key: String,
    pub checksum_sha256: String,
    pub status: DocumentStatus,
    pub metadata: Option<serde_json::Value>,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(
        operator_id: Uuid,
        document_type: DocumentType,
        filename: String,
        content_type: String,
        file_size_bytes: i64,
        storage_key: String,
        checksum_sha256: String,
        uploaded_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            document_id: Uuid::now_v7(),
            operator_id,
            document_type,
            filename,
            content_type,
            file_size_bytes,
            storage_key,
            checksum_sha256,
            status: DocumentStatus::Uploaded,
            metadata: None,
            uploaded_by,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn mark_processing(&mut self) {
        self.status = DocumentStatus::Processing;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self, metadata: serde_json::Value) {
        self.status = DocumentStatus::Completed;
        self.metadata = Some(metadata);
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: &str) {
        self.status = DocumentStatus::Failed;
        self.metadata = Some(serde_json::json!({"error": error}));
        self.updated_at = Utc::now();
    }

    pub fn can_delete(&self) -> bool {
        matches!(self.status, DocumentStatus::Uploaded | DocumentStatus::Failed)
    }

    pub fn max_file_size_bytes() -> i64 {
        10 * 1024 * 1024 // 10MB per SRS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document_is_uploaded() {
        let doc = Document::new(
            Uuid::now_v7(), DocumentType::TradeLicense, "license.pdf".into(),
            "application/pdf".into(), 1024, "bucket/license.pdf".into(),
            "abc123".into(), Uuid::now_v7(),
        );
        assert_eq!(doc.status, DocumentStatus::Uploaded);
        assert!(doc.can_delete());
    }

    #[test]
    fn test_mark_processing() {
        let mut doc = Document::new(
            Uuid::now_v7(), DocumentType::TradeLicense, "license.pdf".into(),
            "application/pdf".into(), 1024, "bucket/license.pdf".into(),
            "abc123".into(), Uuid::now_v7(),
        );
        doc.mark_processing();
        assert_eq!(doc.status, DocumentStatus::Processing);
        assert!(!doc.can_delete());
    }

    #[test]
    fn test_mark_completed() {
        let mut doc = Document::new(
            Uuid::now_v7(), DocumentType::TradeLicense, "license.pdf".into(),
            "application/pdf".into(), 1024, "bucket/license.pdf".into(),
            "abc123".into(), Uuid::now_v7(),
        );
        doc.mark_completed(serde_json::json!({"ocr_text": "Trade License"}));
        assert_eq!(doc.status, DocumentStatus::Completed);
        assert!(doc.metadata.is_some());
    }

    #[test]
    fn test_mark_failed() {
        let mut doc = Document::new(
            Uuid::now_v7(), DocumentType::TradeLicense, "license.pdf".into(),
            "application/pdf".into(), 1024, "bucket/license.pdf".into(),
            "abc123".into(), Uuid::now_v7(),
        );
        doc.mark_failed("OCR failed");
        assert_eq!(doc.status, DocumentStatus::Failed);
        assert!(doc.can_delete());
    }

    #[test]
    fn test_max_file_size() {
        assert_eq!(Document::max_file_size_bytes(), 10 * 1024 * 1024);
    }
}
