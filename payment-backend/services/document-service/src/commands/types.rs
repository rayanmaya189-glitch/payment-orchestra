//! Command types for BC-13 Document Management

use uuid::Uuid;
use crate::domain::*;

/// Upload a new document with blob content.
pub struct UploadDocumentCommand {
    pub operator_id: Uuid,
    pub uploaded_by: Uuid,
    pub category: DocumentCategory,
    pub filename: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Start OCR processing for a document.
pub struct StartOcrCommand {
    pub document_id: Uuid,
}

/// Complete OCR processing with extracted text.
pub struct CompleteOcrCommand {
    pub document_id: Uuid,
    pub ocr_result: String,
}

/// Mark OCR as failed.
pub struct FailOcrCommand {
    pub document_id: Uuid,
    pub error: String,
}

/// Delete a document record.
pub struct DeleteDocumentCommand {
    pub document_id: Uuid,
}
