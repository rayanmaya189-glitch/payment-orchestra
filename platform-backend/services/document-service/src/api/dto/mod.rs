use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct UploadDocumentRequest {
    pub operator_id: Uuid,
    pub document_type: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: Option<i64>,
    pub file_data: String, // base64-encoded
}

#[derive(Debug, Serialize)]
pub struct UploadDocumentResponse {
    pub document_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentDetailResponse {
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub document_type: String,
    pub status: String,
    pub verification: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub file_hash: String,
    pub uploaded_by: String,
    pub verified_by: Option<String>,
    pub verification_notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyDocumentRequest {
    pub approved: bool,
    pub notes: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyDocumentResponse {
    pub status: String,
    pub approved: bool,
    pub verified_by: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}
