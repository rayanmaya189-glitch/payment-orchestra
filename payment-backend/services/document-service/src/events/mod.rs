//! Document Management domain events — BC-13

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentEvent {
    Uploaded(DocumentUploaded),
    OcrCompleted(DocumentOcrCompleted),
    OcrFailed(DocumentOcrFailed),
}

pub const EVENT_TYPE_UPLOADED: &str = "document.uploaded";
pub const EVENT_TYPE_OCR_COMPLETED: &str = "document.ocr_completed";
pub const EVENT_TYPE_OCR_FAILED: &str = "document.ocr_failed";

impl DocumentEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Uploaded(_) => EVENT_TYPE_UPLOADED,
            Self::OcrCompleted(_) => EVENT_TYPE_OCR_COMPLETED,
            Self::OcrFailed(_) => EVENT_TYPE_OCR_FAILED,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUploaded {
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub category: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOcrCompleted {
    pub document_id: Uuid,
    pub ocr_result: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOcrFailed {
    pub document_id: Uuid,
    pub error: String,
    pub occurred_at: DateTime<Utc>,
}
