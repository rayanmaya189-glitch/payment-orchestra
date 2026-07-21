use uuid::Uuid;

/// Command to upload a new document.
#[derive(Debug, Clone)]
pub struct UploadDocumentCommand {
    pub operator_id: Uuid,
    pub document_type: String,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub file_data: Vec<u8>,
    pub uploaded_by: String,
}

/// Command to verify or reject a document.
#[derive(Debug, Clone)]
pub struct VerifyDocumentCommand {
    pub document_id: Uuid,
    pub notes: String,
    pub approved: bool,
    pub verified_by: String,
}

/// Command to get a single document.
#[derive(Debug, Clone)]
pub struct GetDocumentQuery {
    pub document_id: Uuid,
}

/// Command to list documents for an operator.
#[derive(Debug, Clone)]
pub struct ListDocumentsQuery {
    pub operator_id: Uuid,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Command to delete a document.
#[derive(Debug, Clone)]
pub struct DeleteDocumentCommand {
    pub document_id: Uuid,
    pub deleted_by: String,
}
