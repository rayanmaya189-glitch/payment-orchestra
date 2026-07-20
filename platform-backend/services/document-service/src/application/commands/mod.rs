use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct UploadDocumentCommand { pub operator_id: Uuid, pub document_type: String, pub filename: String, pub content_type: String, pub file_size: i64, pub file_data: Vec<u8>, pub uploaded_by: String }
#[derive(Debug, Clone)]
pub struct VerifyDocumentCommand { pub document_id: Uuid, pub notes: String, pub approved: bool }
