use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateKybCaseRequest {
    pub operator_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UploadDocumentRequest {
    pub document_type: String,
    pub file_key: String,
    pub file_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct DecideCaseRequest {
    pub decision: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct KybCaseResponse {
    pub id: Uuid,
    pub operator_id: Uuid,
    pub status: String,
    pub assigned_officer: Option<Uuid>,
    pub document_count: usize,
    pub verified_count: usize,
    pub risk_score: Option<f64>,
    pub submitted_at: String,
    pub decided_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}
