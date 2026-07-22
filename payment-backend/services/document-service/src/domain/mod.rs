//! Document Management domain model — BC-13
//!
//! MinIO-backed blob storage with metadata and OCR pipeline trigger.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// DocumentStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentStatus {
    /// File uploaded to storage, awaiting processing.
    Uploaded,
    /// OCR pipeline is processing the document.
    OcrProcessing,
    /// OCR extraction completed successfully.
    OcrCompleted,
    /// OCR extraction failed.
    OcrFailed,
}

impl std::fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uploaded => write!(f, "uploaded"),
            Self::OcrProcessing => write!(f, "ocr_processing"),
            Self::OcrCompleted => write!(f, "ocr_completed"),
            Self::OcrFailed => write!(f, "ocr_failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentCategory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCategory {
    /// KYB/KYC evidence (business license, ID).
    KybEvidence,
    /// Settlement advice file from acquirer.
    SettlementAdvice,
    /// Invoice document.
    Invoice,
    /// Representment evidence for chargeback.
    RepresentmentEvidence,
    /// General document (uncategorized).
    General,
}

impl std::fmt::Display for DocumentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KybEvidence => write!(f, "kyb_evidence"),
            Self::SettlementAdvice => write!(f, "settlement_advice"),
            Self::Invoice => write!(f, "invoice"),
            Self::RepresentmentEvidence => write!(f, "representment_evidence"),
            Self::General => write!(f, "general"),
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentRecord aggregate
// ---------------------------------------------------------------------------

/// Core DocumentRecord aggregate root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub uploaded_by: Uuid,
    pub category: DocumentCategory,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub status: DocumentStatus,
    pub ocr_result: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl DocumentRecord {
    /// Max allowed file size: 10MB.
    pub const MAX_SIZE_BYTES: i64 = 10 * 1024 * 1024;

    /// Allowed content types.
    pub const ALLOWED_CONTENT_TYPES: &[&'static str] = &[
        "application/pdf",
        "image/png",
        "image/jpeg",
        "image/tiff",
        "application/csv",
        "application/json",
        "text/plain",
        "application/xml",
    ];

    /// Create a new document record in `Uploaded` status.
    pub fn new(
        operator_id: Uuid,
        uploaded_by: Uuid,
        category: DocumentCategory,
        filename: String,
        content_type: String,
        size_bytes: i64,
        storage_key: String,
    ) -> Result<Self, DocumentError> {
        // Validate file size
        if size_bytes > Self::MAX_SIZE_BYTES {
            return Err(DocumentError::TooLarge(size_bytes));
        }

        // Validate content type
        if !Self::ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
            return Err(DocumentError::UnsupportedContentType(content_type));
        }

        Ok(Self {
            document_id: Uuid::now_v7(),
            operator_id,
            uploaded_by,
            category,
            filename,
            content_type,
            size_bytes,
            storage_key,
            status: DocumentStatus::Uploaded,
            ocr_result: None,
            created_at: Utc::now(),
        })
    }

    /// Mark document as OCR processing.
    pub fn start_ocr(&mut self) -> Result<(), DocumentError> {
        if self.status != DocumentStatus::Uploaded {
            return Err(DocumentError::InvalidStatusTransition);
        }
        self.status = DocumentStatus::OcrProcessing;
        Ok(())
    }

    /// Mark OCR as completed with extracted text.
    pub fn complete_ocr(&mut self, result: String) -> Result<(), DocumentError> {
        if self.status != DocumentStatus::OcrProcessing {
            return Err(DocumentError::InvalidStatusTransition);
        }
        self.status = DocumentStatus::OcrCompleted;
        self.ocr_result = Some(result);
        Ok(())
    }

    /// Mark OCR as failed.
    pub fn fail_ocr(&mut self, error: &str) -> Result<(), DocumentError> {
        if self.status != DocumentStatus::OcrProcessing {
            return Err(DocumentError::InvalidStatusTransition);
        }
        self.status = DocumentStatus::OcrFailed;
        self.ocr_result = Some(format!("OCR error: {}", error));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum DocumentError {
    #[error("Document not found: {0}")]
    NotFound(Uuid),
    #[error("Document too large: {0} bytes (max 10MB)")]
    TooLarge(i64),
    #[error("Unsupported content type: {0}")]
    UnsupportedContentType(String),
    #[error("Invalid document status transition")]
    InvalidStatusTransition,
    #[error("OCR processing failed")]
    OcrFailed,
}
