//! PostgreSQL-backed DocumentRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::DocumentRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as DocumentActiveModel,
    Column as DocumentColumn,
    Entity as DocumentEntity,
    Model as DocumentModel,
};

pub struct PostgresDocumentRepository {
    pub db: DatabaseConnection,
}

impl PostgresDocumentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DocumentRepository for PostgresDocumentRepository {
    async fn load(&self, id: Uuid) -> Result<Option<DocumentRecord>, DocumentError> {
        let result = DocumentEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DocumentError::NotFound(id))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, doc: &DocumentRecord) -> Result<(), DocumentError> {
        let model = domain_to_model(doc);
        let exists = DocumentEntity::find_by_id(doc.document_id)
            .one(&self.db)
            .await
            .map_err(|e| DocumentError::NotFound(doc.document_id))?
            .is_some();

        if exists {
            DocumentEntity::update(DocumentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| DocumentError::NotFound(doc.document_id))?;
        } else {
            DocumentEntity::insert(DocumentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| DocumentError::NotFound(doc.document_id))?;
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DocumentError> {
        DocumentEntity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| DocumentError::NotFound(id))?;
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<DocumentRecord>, DocumentError> {
        let models = DocumentEntity::find()
            .filter(DocumentColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| DocumentError::NotFound(operator_id))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_type(&self, operator_id: Uuid, category: &str) -> Result<Vec<DocumentRecord>, DocumentError> {
        let models = DocumentEntity::find()
            .filter(DocumentColumn::OperatorId.eq(operator_id))
            .filter(DocumentColumn::Category.eq(category))
            .all(&self.db)
            .await
            .map_err(|e| DocumentError::NotFound(operator_id))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

fn domain_to_model(d: &DocumentRecord) -> DocumentModel {
    DocumentModel {
        document_id: d.document_id,
        operator_id: d.operator_id,
        uploaded_by: d.uploaded_by,
        category: d.category.as_str().to_string(),
        filename: d.filename.clone(),
        content_type: d.content_type.clone(),
        size_bytes: d.size_bytes,
        storage_key: d.storage_key.clone(),
        status: d.status.as_str().to_string(),
        ocr_result: d.ocr_result.clone(),
        created_at: d.created_at,
    }
}

fn model_to_domain(m: DocumentModel) -> Result<DocumentRecord, DocumentError> {
    let category = DocumentCategory::from_str(&m.category)
        .ok_or_else(|| DocumentError::NotFound(m.document_id))?;
    let status = DocumentStatus::from_str(&m.status)
        .ok_or_else(|| DocumentError::NotFound(m.document_id))?;

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
