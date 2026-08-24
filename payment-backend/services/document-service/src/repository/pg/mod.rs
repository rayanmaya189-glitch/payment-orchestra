//! PostgreSQL repository for Document Service — BC-13
//!
//! Persists [`DocumentRecord`] to the `documents` table.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::document::{self, Entity as DocumentEntity, Column as DocumentColumn};
use crate::repository::DocumentRepository;

#[derive(Clone)]
pub struct PostgresDocumentRepository {
    db: sea_orm::DatabaseConnection,
}

impl PostgresDocumentRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DocumentRepository for PostgresDocumentRepository {
    async fn load(&self, id: Uuid) -> Result<Option<DocumentRecord>, DocumentError> {
        let result = DocumentEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DocumentError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, record: &DocumentRecord) -> Result<(), DocumentError> {
        let model = domain_to_model(record);

        document::Entity::insert(model.clone())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(document::Column::DocumentId)
                    .update_columns([
                        document::Column::Category,
                        document::Column::Filename,
                        document::Column::ContentType,
                        document::Column::SizeBytes,
                        document::Column::StorageKey,
                        document::Column::Status,
                        document::Column::OcrResult,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| DocumentError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DocumentError> {
        DocumentEntity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| DocumentError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_type(&self, operator_id: Uuid, category: &str) -> Result<Vec<DocumentRecord>, DocumentError> {
        let results = DocumentEntity::find()
            .filter(DocumentColumn::OperatorId.eq(operator_id))
            .filter(DocumentColumn::Category.eq(category))
            .order_by_desc(DocumentColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| DocumentError::DatabaseError(e.to_string()))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError> {
        let results = DocumentEntity::find()
            .filter(DocumentColumn::OperatorId.eq(operator_id))
            .order_by_desc(DocumentColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| DocumentError::DatabaseError(e.to_string()))?;

        results.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain <-> Entity conversion helpers ─────────────────────────────────────

fn domain_to_model(record: &DocumentRecord) -> document::ActiveModel {
    document::ActiveModel {
        document_id: sea_orm::ActiveValue::Set(record.document_id),
        operator_id: sea_orm::ActiveValue::Set(record.operator_id),
        uploaded_by: sea_orm::ActiveValue::Set(record.uploaded_by),
        category: sea_orm::ActiveValue::Set(record.category.to_string()),
        filename: sea_orm::ActiveValue::Set(record.filename.clone()),
        content_type: sea_orm::ActiveValue::Set(record.content_type.clone()),
        size_bytes: sea_orm::ActiveValue::Set(record.size_bytes),
        storage_key: sea_orm::ActiveValue::Set(record.storage_key.clone()),
        status: sea_orm::ActiveValue::Set(record.status.to_string()),
        ocr_result: sea_orm::ActiveValue::Set(record.ocr_result.clone()),
        created_at: sea_orm::ActiveValue::Set(record.created_at),
    }
}

fn model_to_domain(m: document::Model) -> Result<DocumentRecord, DocumentError> {
    let category = match m.category.as_str() {
        "kyb_evidence" => DocumentCategory::KybEvidence,
        "settlement_advice" => DocumentCategory::SettlementAdvice,
        "invoice" => DocumentCategory::Invoice,
        "representment_evidence" => DocumentCategory::RepresentmentEvidence,
        "general" => DocumentCategory::General,
        other => return Err(DocumentError::DatabaseError(format!("Invalid category: {other}"))),
    };

    let status = match m.status.as_str() {
        "uploaded" => DocumentStatus::Uploaded,
        "ocr_processing" => DocumentStatus::OcrProcessing,
        "ocr_completed" => DocumentStatus::OcrCompleted,
        "ocr_failed" => DocumentStatus::OcrFailed,
        other => return Err(DocumentError::DatabaseError(format!("Invalid status: {other}"))),
    };

    Ok(DocumentRecord {
        document_id: m.document_id,
        operator_id: m.operator_id,
        uploaded_by: m.uploaded_by,
        category,
        filename: m.filename,
        content_type: m.content_type,
        size_bytes: m.size_bytes,
        storage_key: m.storage_key,
        status,
        ocr_result: m.ocr_result,
        created_at: m.created_at,
    })
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_record() -> DocumentRecord {
        DocumentRecord {
            document_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            uploaded_by: Uuid::now_v7(),
            category: DocumentCategory::KybEvidence,
            filename: "license.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: 1024,
            storage_key: "operator_1/doc_123_license.pdf".into(),
            status: DocumentStatus::Uploaded,
            ocr_result: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_domain_to_model() {
        let rec = sample_record();
        let model = domain_to_model(&rec);

        assert_eq!(model.document_id.unwrap(), rec.document_id);
        assert_eq!(model.category.unwrap(), "kyb_evidence");
        assert_eq!(model.status.unwrap(), "uploaded");
        assert_eq!(model.filename.unwrap(), "license.pdf");
        assert_eq!(model.size_bytes.unwrap(), 1024);
    }

    #[tokio::test]
    async fn test_model_to_domain() {
        let rec = sample_record();
        let entity = document::Model {
            document_id: rec.document_id,
            operator_id: rec.operator_id,
            uploaded_by: rec.uploaded_by,
            category: "invoice".into(),
            filename: "invoice.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: 2048,
            storage_key: "key".into(),
            status: "ocr_completed".into(),
            ocr_result: Some("extracted text".into()),
            created_at: rec.created_at,
        };

        let domain = model_to_domain(entity).unwrap();
        assert_eq!(domain.document_id, rec.document_id);
        assert_eq!(domain.category, DocumentCategory::Invoice);
        assert_eq!(domain.status, DocumentStatus::OcrCompleted);
        assert_eq!(domain.ocr_result, Some("extracted text".into()));
        assert_eq!(domain.size_bytes, 2048);
    }

    #[tokio::test]
    async fn test_invalid_category() {
        let rec = sample_record();
        let entity = document::Model {
            document_id: rec.document_id,
            operator_id: rec.operator_id,
            uploaded_by: rec.uploaded_by,
            category: "nonexistent".into(),
            filename: "x".into(),
            content_type: "text/plain".into(),
            size_bytes: 100,
            storage_key: "key".into(),
            status: "uploaded".into(),
            ocr_result: None,
            created_at: rec.created_at,
        };

        let result = model_to_domain(entity);
        assert!(result.is_err());
        assert!(matches!(result, Err(DocumentError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn test_all_statuses_roundtrip() {
        let rec = sample_record();
        for (status_str, status) in [
            ("uploaded", DocumentStatus::Uploaded),
            ("ocr_processing", DocumentStatus::OcrProcessing),
            ("ocr_completed", DocumentStatus::OcrCompleted),
            ("ocr_failed", DocumentStatus::OcrFailed),
        ] {
            let entity = document::Model {
                document_id: rec.document_id,
                operator_id: rec.operator_id,
                uploaded_by: rec.uploaded_by,
                category: "kyb_evidence".into(),
                filename: "doc.pdf".into(),
                content_type: "application/pdf".into(),
                size_bytes: 100,
                storage_key: "key".into(),
                status: status_str.into(),
                ocr_result: None,
                created_at: rec.created_at,
            };
            let domain = model_to_domain(entity).unwrap();
            assert_eq!(domain.status, status);
        }
    }
}
