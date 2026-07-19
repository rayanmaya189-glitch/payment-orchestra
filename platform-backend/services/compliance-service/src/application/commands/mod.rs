use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateKybCaseCommand {
    pub operator_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct UploadDocumentCommand {
    pub kyb_case_id: Uuid,
    pub document_type: String,
    pub file_key: String,
    pub file_hash: String,
}

#[derive(Debug, Clone)]
pub struct VerifyDocumentCommand {
    pub document_id: Uuid,
    pub verified: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AssignOfficerCommand {
    pub kyb_case_id: Uuid,
    pub officer_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct DecideKybCaseCommand {
    pub kyb_case_id: Uuid,
    pub decision: String, // approved | rejected | suspended
    pub reason: String,
    pub decided_by: Uuid,
}

#[derive(Debug, Clone)]
pub struct RequestDocumentsCommand {
    pub kyb_case_id: Uuid,
    pub requested_documents: Vec<String>,
    pub reason: String,
}
