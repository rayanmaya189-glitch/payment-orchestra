use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresReconciliationRepository;
use crate::repository::SettlementBatchRepository;
use crate::domain::*;
use crate::entities::settlement_batch::{
    Entity as SettlementBatchEntity,
    ActiveModel as SettlementBatchActiveModel,
    Model as SettlementBatchModel,
    Column as SettlementBatchColumn,
};

#[async_trait]
impl SettlementBatchRepository for PostgresReconciliationRepository {
    async fn load_settlement_batch(
        &self,
        id: Uuid,
    ) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let result = SettlementBatchEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_batch_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_settlement_batch(
        &self,
        batch: &SettlementBatch,
    ) -> Result<(), ReconciliationError> {
        let model = settlement_batch_domain_to_model(batch)?;
        let exists = SettlementBatchEntity::find_by_id(batch.settlement_batch_id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            SettlementBatchEntity::update(SettlementBatchActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        } else {
            SettlementBatchEntity::insert(SettlementBatchActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_batch_by_checksum(
        &self,
        checksum: &str,
    ) -> Result<Option<SettlementBatch>, ReconciliationError> {
        let result = SettlementBatchEntity::find()
            .filter(SettlementBatchColumn::FileChecksum.eq(checksum))
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_batch_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn list_all_batches(&self) -> Result<Vec<SettlementBatch>, ReconciliationError> {
        let models = SettlementBatchEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        let mut batches: Vec<SettlementBatch> = models
            .into_iter()
            .map(settlement_batch_model_to_domain)
            .collect::<Result<Vec<_>, _>>()?;
        batches.sort_by_key(|a| a.ingested_at);
        Ok(batches)
    }
}

fn settlement_batch_domain_to_model(batch: &SettlementBatch) -> Result<SettlementBatchModel, ReconciliationError> {
    Ok(SettlementBatchModel {
        settlement_batch_id: batch.settlement_batch_id,
        batch_file_name: batch.file_checksum.clone(),
        status: batch.status.to_string(),
        ingested_at: batch.ingested_at,
        total_transactions: batch.total_records,
        total_amount_minor: batch.total_amount_minor,
        currency: "AED".to_string(),
        records: serde_json::to_value(&batch.records)
            .map_err(|e| ReconciliationError::Validation(format!("Serialize records: {}", e)))?,
        file_checksum: batch.file_checksum.clone(),
        matched_at: None,
        quarantined_at: None,
    })
}

fn settlement_batch_model_to_domain(m: SettlementBatchModel) -> Result<SettlementBatch, ReconciliationError> {
    let status = match m.status.as_str() {
        "pending" => BatchStatus::Ingesting,
        "matched" => BatchStatus::Processed,
        "exception" => BatchStatus::Quarantined,
        _ => BatchStatus::Ingesting,
    };

    let records: Vec<SettlementRecord> = serde_json::from_value(m.records)
        .map_err(|e| ReconciliationError::Validation(format!("Deserialize records: {}", e)))?;

    Ok(SettlementBatch {
        settlement_batch_id: m.settlement_batch_id,
        operator_id: Uuid::default(),
        acquirer_link_id: Uuid::default(),
        file_checksum: m.file_checksum,
        file_format: "csv".to_string(),
        status,
        total_records: m.total_transactions,
        matched_count: 0,
        unmatched_count: 0,
        total_amount_minor: m.total_amount_minor,
        records,
        ingested_at: m.ingested_at,
        processed_at: m.matched_at,
    })
}
