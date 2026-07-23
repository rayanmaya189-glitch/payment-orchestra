//! gRPC service implementation for document-service (BC-13).
//! Translates between protobuf types and domain types for document lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::{self, DocumentCategory, DocumentError, DocumentStatus};
use crate::queries::QueryHandler;

use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationRequest, PaginationResponse};
use platform_proto::document::document_service_server::DocumentService;
use platform_proto::document::*;

pub struct DocumentGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> DocumentGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> DocumentService for DocumentGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn upload_document(
        &self,
        request: Request<UploadDocumentRequest>,
    ) -> Result<Response<UploadDocumentResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let category = match req.category.as_str() {
            "kyb" => DocumentCategory::KybEvidence,
            "invoice" => DocumentCategory::Invoice,
            "dispute_evidence" => DocumentCategory::RepresentmentEvidence,
            "contract" => DocumentCategory::General,
            _ => DocumentCategory::General,
        };

        if req.file_content.is_empty() {
            return Err(Status::invalid_argument("file_content is required"));
        }
        if req.file_name.is_empty() {
            return Err(Status::invalid_argument("file_name is required"));
        }

        let cmd = commands::UploadDocumentCommand {
            operator_id,
            uploaded_by: operator_id,
            category,
            filename: req.file_name,
            content_type: if req.content_type.is_empty() {
                "application/octet-stream".to_string()
            } else {
                req.content_type
            },
            body: req.file_content,
        };

        match self.commands.upload_document(cmd).await {
            Ok(record) => {
                Ok(Response::new(UploadDocumentResponse {
                    document_id: record.document_id.to_string(),
                    status: record.status.to_string(),
                    uploaded_at: Some(Timestamp {
                        unix_ms: record.created_at.timestamp_millis(),
                    }),
                    file_size_bytes: record.size_bytes,
                }))
            }
            Err(e) => Err(document_error_to_status(e)),
        }
    }

    async fn get_document_url(
        &self,
        request: Request<GetDocumentUrlRequest>,
    ) -> Result<Response<GetDocumentUrlResponse>, Status> {
        let req = request.into_inner();
        let document_id = parse_uuid(&req.document_id, "document_id")?;

        match self.queries.get_document(document_id).await {
            Ok(record) => {
                // In production, this would generate a pre-signed MinIO URL.
                // For Phase 1, return a storage-key based URL.
                Ok(Response::new(GetDocumentUrlResponse {
                    url: format!("/api/v1/documents/{}/download", document_id),
                    expires_at_unix_ms: (chrono::Utc::now()
                        + chrono::Duration::hours(1))
                    .timestamp_millis(),
                }))
            }
            Err(e) => Err(document_error_to_status(e)),
        }
    }

    async fn list_documents(
        &self,
        request: Request<ListDocumentsRequest>,
    ) -> Result<Response<ListDocumentsResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let page_limit = req.pagination.as_ref().map_or(50, |p| {
            if p.limit > 0 && p.limit <= 100 { p.limit as usize } else { 50 }
        });
        let cursor = req.pagination.as_ref().and_then(|p| {
            if p.cursor.is_empty() { None } else { Some(p.cursor.clone()) }
        });

        let records = if req.category.is_empty() {
            self.queries.find_by_operator(operator_id).await
        } else {
            self.queries.find_by_type(operator_id, &req.category).await
        };

        match records {
            Ok(all_records) => {
                let filtered: Vec<domain::DocumentRecord> = all_records
                    .into_iter()
                    .skip_while(|doc| {
                        if let Some(ref c) = cursor {
                            doc.document_id.to_string() != *c
                        } else {
                            false
                        }
                    })
                    .take(page_limit)
                    .collect();

                let has_more = filtered.len() >= page_limit;
                let next_cursor = filtered.last().map(|doc| doc.document_id.to_string());
                let views: Vec<DocumentView> =
                    filtered.into_iter().map(document_record_to_view).collect();

                Ok(Response::new(ListDocumentsResponse {
                    documents: views,
                    pagination: Some(PaginationResponse {
                        next_cursor: next_cursor.unwrap_or_default(),
                        has_more,
                        as_of_unix_ms: chrono::Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(document_error_to_status(e)),
        }
    }

    async fn delete_document(
        &self,
        request: Request<DeleteDocumentRequest>,
    ) -> Result<Response<DeleteDocumentResponse>, Status> {
        let req = request.into_inner();
        let document_id = parse_uuid(&req.document_id, "document_id")?;

        let cmd = commands::DeleteDocumentCommand { document_id };

        match self.commands.delete_document(cmd).await {
            Ok(()) => Ok(Response::new(DeleteDocumentResponse { deleted: true })),
            Err(e) => Err(document_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn document_record_to_view(record: domain::DocumentRecord) -> DocumentView {
    let (ocr_status, ocr_extracted_text) = match record.status {
        DocumentStatus::Uploaded => ("pending".to_string(), String::new()),
        DocumentStatus::OcrProcessing => ("processing".to_string(), String::new()),
        DocumentStatus::OcrCompleted => (
            "completed".to_string(),
            record.ocr_result.clone().unwrap_or_default(),
        ),
        DocumentStatus::OcrFailed => (
            "failed".to_string(),
            record.ocr_result.clone().unwrap_or_default(),
        ),
    };

    DocumentView {
        document_id: record.document_id.to_string(),
        file_name: record.filename,
        content_type: record.content_type,
        category: record.category.to_string(),
        status: record.status.to_string(),
        file_size_bytes: record.size_bytes,
        uploaded_at: Some(Timestamp {
            unix_ms: record.created_at.timestamp_millis(),
        }),
        ocr_status,
        ocr_extracted_text,
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn document_error_to_status(e: DocumentError) -> Status {
    match e {
        DocumentError::NotFound(id) => {
            Status::not_found(format!("Document not found: {}", id))
        }
        DocumentError::TooLarge(size) => {
            Status::invalid_argument(format!("Document too large: {} bytes (max 10MB)", size))
        }
        DocumentError::UnsupportedContentType(ct) => {
            Status::invalid_argument(format!("Unsupported content type: {}", ct))
        }
        DocumentError::InvalidStatusTransition => {
            Status::failed_precondition("Invalid document status transition")
        }
        DocumentError::OcrFailed => {
            Status::internal("OCR processing failed")
        }
    }
}

impl From<DocumentError> for Status {
    fn from(e: DocumentError) -> Self {
        document_error_to_status(e)
    }
}
