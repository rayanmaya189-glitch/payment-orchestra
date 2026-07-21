use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use crate::application::commands::*;
use crate::domain::aggregates::Document;
use crate::domain::rules::{DocumentRepository, StorageProvider};
use crate::domain::value_objects::{DocumentError, DocumentType};
use crate::infrastructure::messaging::EventPublisher;
use platform_error::PlatformError;
use sha2::{Sha256, Digest};
use shared_types::events::EventEnvelope;

pub struct DocumentServiceImpl {
    repo: Box<dyn DocumentRepository>,
    storage: Box<dyn StorageProvider>,
    db: DatabaseConnection,
    publisher: Option<EventPublisher>,
}

impl DocumentServiceImpl {
    pub fn new(
        repo: Box<dyn DocumentRepository>,
        storage: Box<dyn StorageProvider>,
        db: DatabaseConnection,
    ) -> Self {
        Self {
            repo,
            storage,
            db,
            publisher: None,
        }
    }

    pub fn with_publisher(mut self, publisher: EventPublisher) -> Self {
        self.publisher = Some(publisher);
        self
    }

    async fn publish_event(
        &self,
        doc: &Document,
        event_type: &str,
        extra: serde_json::Value,
    ) {
        if let Some(ref publisher) = self.publisher {
            let mut payload = serde_json::json!({
                "document_id": doc.document_id,
                "operator_id": doc.operator_id,
                "document_type": doc.document_type,
                "status": doc.status.as_str(),
                "verification": doc.verification_status.as_str(),
            });
            if let Some(obj) = payload.as_object_mut() {
                if let Some(extra_obj) = extra.as_object() {
                    for (k, v) in extra_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }

            let event = EventEnvelope::new(
                "Document",
                doc.document_id,
                event_type,
                "document_service",
                doc.document_id,
                payload,
            );

            if let Err(e) = publisher.publish(&event).await {
                tracing::warn!("Failed to publish event {event_type}: {e}");
            }
        }
    }
}

#[async_trait]
pub trait DocumentService: Send + Sync {
    async fn upload(&self, cmd: UploadDocumentCommand) -> Result<Uuid, PlatformError>;
    async fn verify(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError>;
    async fn get(&self, id: Uuid) -> Result<Document, PlatformError>;
    async fn list_by_operator(
        &self,
        query: ListDocumentsQuery,
    ) -> Result<Vec<Document>, PlatformError>;
}

#[async_trait]
impl DocumentService for DocumentServiceImpl {
    async fn upload(&self, cmd: UploadDocumentCommand) -> Result<Uuid, PlatformError> {
        // Validate document type
        let doc_type: DocumentType = cmd
            .document_type
            .parse()
            .map_err(|e: String| PlatformError::Validation(
                platform_error::ValidationError::MissingField(e),
            ))?;

        // Validate file size
        if cmd.file_size > doc_type.max_file_size() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(format!(
                    "File size {} exceeds max {} for {}",
                    cmd.file_size,
                    doc_type.max_file_size(),
                    doc_type.as_str()
                )),
            ));
        }

        // Validate content type
        let allowed = doc_type.allowed_content_types();
        if !allowed.contains(&cmd.content_type.as_str()) {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::MissingField(format!(
                    "Content type {} not allowed for {}. Allowed: {:?}",
                    cmd.content_type,
                    doc_type.as_str(),
                    allowed
                )),
            ));
        }

        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&cmd.file_data);
        let hash = hex::encode(hasher.finalize());

        // Generate storage key
        let storage_key = format!(
            "{}/{}/{}",
            cmd.operator_id,
            cmd.document_type,
            uuid::Uuid::now_v7()
        );

        // Upload to storage
        let storage_url = self
            .storage
            .upload(&storage_key, &cmd.file_data, &cmd.content_type)
            .await?;

        // Create document aggregate
        let mut doc = Document::new(
            cmd.operator_id,
            cmd.document_type,
            cmd.filename,
            cmd.content_type,
            cmd.file_size,
            storage_key,
            hash,
            cmd.uploaded_by,
        );

        doc.set_metadata(serde_json::json!({
            "storage_url": storage_url,
            "original_filename": doc.filename,
        }));

        self.repo.save(&doc).await?;
        self.publish_event(&doc, "DocumentUploaded", serde_json::json!({}))
            .await;

        Ok(doc.document_id)
    }

    async fn verify(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError> {
        let mut doc = self
            .repo
            .find_by_id(cmd.document_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "document".into(),
                id: cmd.document_id,
            })?;

        // ABAC check: verify caller has compliance_officer or platform_admin role
        // This is enforced at the route level via AuthPrincipal extraction.
        // Here we just validate the state transition.

        if cmd.approved {
            doc.verify(&cmd.notes, &cmd.verified_by)
                .map_err(|e| PlatformError::Validation(
                    platform_error::ValidationError::MissingField(e.to_string()),
                ))?;
        } else {
            doc.reject(&cmd.notes, &cmd.verified_by)
                .map_err(|e| PlatformError::Validation(
                    platform_error::ValidationError::MissingField(e.to_string()),
                ))?;
        }

        self.repo.save(&doc).await?;
        self.publish_event(
            &doc,
            if cmd.approved {
                "DocumentVerified"
            } else {
                "DocumentRejected"
            },
            serde_json::json!({"notes": cmd.notes}),
        )
        .await;

        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Document, PlatformError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "document".into(),
                id,
            })
    }

    async fn list_by_operator(
        &self,
        query: ListDocumentsQuery,
    ) -> Result<Vec<Document>, PlatformError> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        self.repo
            .find_by_operator(query.operator_id, limit, offset)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{DocumentStatus, VerificationStatus};
    use crate::infrastructure::adapters::noop_document_repository::NoopDocumentRepository;
    use crate::infrastructure::adapters::local_storage::LocalStorageProvider;

    fn make_service() -> DocumentServiceImpl {
        let repo = NoopDocumentRepository::new();
        let storage = LocalStorageProvider::new("/tmp/test-doc-storage".into());
        let db = sea_orm::DatabaseConnection::default();
        DocumentServiceImpl::new(Box::new(repo), Box::new(storage), db)
    }

    #[tokio::test]
    async fn test_upload_document() {
        let svc = make_service();
        let cmd = UploadDocumentCommand {
            operator_id: Uuid::now_v7(),
            document_type: "trade_license".into(),
            filename: "license.pdf".into(),
            content_type: "application/pdf".into(),
            file_size: 1024,
            file_data: vec![0u8; 1024],
            uploaded_by: "user1".into(),
        };
        let doc_id = svc.upload(cmd).await.unwrap();
        let doc = svc.get(doc_id).await.unwrap();
        assert_eq!(doc.status, DocumentStatus::Uploaded);
        assert_eq!(doc.verification_status, VerificationStatus::Pending);
    }

    #[tokio::test]
    async fn test_upload_wrong_content_type() {
        let svc = make_service();
        let cmd = UploadDocumentCommand {
            operator_id: Uuid::now_v7(),
            document_type: "trade_license".into(),
            filename: "license.exe".into(),
            content_type: "application/x-executable".into(),
            file_size: 1024,
            file_data: vec![0u8; 1024],
            uploaded_by: "user1".into(),
        };
        let err = svc.upload(cmd).await.unwrap_err();
        assert!(matches!(err, PlatformError::Validation(_)));
    }

    #[tokio::test]
    async fn test_upload_file_too_large() {
        let svc = make_service();
        let cmd = UploadDocumentCommand {
            operator_id: Uuid::now_v7(),
            document_type: "trade_license".into(),
            filename: "big.pdf".into(),
            content_type: "application/pdf".into(),
            file_size: 20 * 1024 * 1024, // 20MB, exceeds 5MB limit
            file_data: vec![0u8; 1024],
            uploaded_by: "user1".into(),
        };
        let err = svc.upload(cmd).await.unwrap_err();
        assert!(matches!(err, PlatformError::Validation(_)));
    }

    #[tokio::test]
    async fn test_verify_document() {
        let svc = make_service();
        let cmd = UploadDocumentCommand {
            operator_id: Uuid::now_v7(),
            document_type: "trade_license".into(),
            filename: "license.pdf".into(),
            content_type: "application/pdf".into(),
            file_size: 1024,
            file_data: vec![0u8; 1024],
            uploaded_by: "user1".into(),
        };
        let doc_id = svc.upload(cmd).await.unwrap();

        svc.verify(VerifyDocumentCommand {
            document_id: doc_id,
            notes: "Document is valid".into(),
            approved: true,
            verified_by: "compliance_officer_1".into(),
        })
        .await
        .unwrap();

        let doc = svc.get(doc_id).await.unwrap();
        assert_eq!(doc.status, DocumentStatus::Verified);
        assert_eq!(doc.verification_status, VerificationStatus::Verified);
    }

    #[tokio::test]
    async fn test_reject_document() {
        let svc = make_service();
        let cmd = UploadDocumentCommand {
            operator_id: Uuid::now_v7(),
            document_type: "trade_license".into(),
            filename: "license.pdf".into(),
            content_type: "application/pdf".into(),
            file_size: 1024,
            file_data: vec![0u8; 1024],
            uploaded_by: "user1".into(),
        };
        let doc_id = svc.upload(cmd).await.unwrap();

        svc.verify(VerifyDocumentCommand {
            document_id: doc_id,
            notes: "Document is blurry".into(),
            approved: false,
            verified_by: "compliance_officer_1".into(),
        })
        .await
        .unwrap();

        let doc = svc.get(doc_id).await.unwrap();
        assert_eq!(doc.status, DocumentStatus::Rejected);
        assert_eq!(doc.verification_status, VerificationStatus::Rejected);
    }

    #[tokio::test]
    async fn test_verify_already_verified_fails() {
        let svc = make_service();
        let cmd = UploadDocumentCommand {
            operator_id: Uuid::now_v7(),
            document_type: "trade_license".into(),
            filename: "license.pdf".into(),
            content_type: "application/pdf".into(),
            file_size: 1024,
            file_data: vec![0u8; 1024],
            uploaded_by: "user1".into(),
        };
        let doc_id = svc.upload(cmd).await.unwrap();

        svc.verify(VerifyDocumentCommand {
            document_id: doc_id,
            notes: "ok".into(),
            approved: true,
            verified_by: "officer".into(),
        })
        .await
        .unwrap();

        let err = svc
            .verify(VerifyDocumentCommand {
                document_id: doc_id,
                notes: "again".into(),
                approved: true,
                verified_by: "officer2".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, PlatformError::Validation(_)));
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let svc = make_service();
        let err = svc.get(Uuid::now_v7()).await.unwrap_err();
        assert!(matches!(err, PlatformError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_list_by_operator() {
        let svc = make_service();
        let operator_id = Uuid::now_v7();

        // Upload two documents
        for i in 0..2 {
            let cmd = UploadDocumentCommand {
                operator_id,
                document_type: "trade_license".into(),
                filename: format!("license_{i}.pdf"),
                content_type: "application/pdf".into(),
                file_size: 1024,
                file_data: vec![0u8; 1024],
                uploaded_by: "user1".into(),
            };
            svc.upload(cmd).await.unwrap();
        }

        let docs = svc
            .list_by_operator(ListDocumentsQuery {
                operator_id,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(docs.len(), 2);
    }
}
