use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;
use crate::domain::aggregates::Document;
use crate::domain::value_objects::{DocumentStatus, VerificationStatus};
use crate::domain::rules::DocumentRepository;
use crate::infrastructure::entities::document_entity;
use platform_error::PlatformError;

pub struct PostgresDocumentRepository { db: DatabaseConnection }
impl PostgresDocumentRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl DocumentRepository for PostgresDocumentRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Document>, PlatformError> {
        let m = document_entity::Entity::find_by_id(id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn save(&self, d: &Document) -> Result<(), PlatformError> {
        let existing = document_entity::Entity::find_by_id(d.document_id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        if let Some(model) = existing {
            let mut a = document_entity::ActiveModel::from(model);
            a.status = Set(d.status.as_str().to_string());
            a.verification_status = Set(d.verification_status.as_str().to_string());
            a.verification_notes = Set(d.verification_notes.clone());
            a.verified_at = Set(d.verified_at.map(|dt| dt.into()));
            a.updated_at = Set(d.updated_at.into());
            a.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        } else {
            let a = document_entity::ActiveModel {
                document_id: Set(d.document_id), operator_id: Set(d.operator_id),
                document_type: Set(d.document_type.clone()), status: Set(d.status.as_str().to_string()),
                filename: Set(d.filename.clone()), content_type: Set(d.content_type.clone()),
                file_size: Set(d.file_size), storage_key: Set(d.storage_key.clone()),
                file_hash: Set(d.file_hash.clone()),
                ocr_result: Set(d.ocr_result.as_ref().and_then(|v| serde_json::to_value(v).ok())),
                metadata: Set(d.metadata.as_ref().and_then(|v| serde_json::to_value(v).ok())),
                uploaded_by: Set(d.uploaded_by.clone()),
                verification_status: Set(d.verification_status.as_str().to_string()),
                verification_notes: Set(d.verification_notes.clone()),
                verified_at: Set(d.verified_at.map(|dt| dt.into())),
                created_at: Set(d.created_at.into()), updated_at: Set(d.updated_at.into()),
            };
            a.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        }
        Ok(())
    }
}

impl From<document_entity::Model> for Document {
    fn from(m: document_entity::Model) -> Self {
        Document {
            document_id: m.document_id, operator_id: m.operator_id,
            document_type: m.document_type, status: DocumentStatus::Uploaded,
            filename: m.filename, content_type: m.content_type, file_size: m.file_size,
            storage_key: m.storage_key, file_hash: m.file_hash,
            ocr_result: m.ocr_result.and_then(|v| serde_json::from_value(v.into()).ok()),
            metadata: m.metadata.and_then(|v| serde_json::from_value(v.into()).ok()),
            uploaded_by: m.uploaded_by,
            verification_status: VerificationStatus::Pending,
            verification_notes: m.verification_notes,
            verified_at: m.verified_at.map(|dt| dt.into()),
            created_at: m.created_at.into(), updated_at: m.updated_at.into(),
        }
    }
}
