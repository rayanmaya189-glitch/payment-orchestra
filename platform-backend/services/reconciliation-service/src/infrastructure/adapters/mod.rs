use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use uuid::Uuid;

use crate::domain::aggregates::{LedgerEntry, SettlementBatch};
use crate::infrastructure::entities::{settlement_batch, ledger_entry};
use crate::infrastructure::repository::{SettlementBatchRepository, LedgerRepository};
use platform_error::PlatformError;

pub struct PostgresSettlementBatchRepository {
    db: DatabaseConnection,
}

impl PostgresSettlementBatchRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SettlementBatchRepository for PostgresSettlementBatchRepository {
    async fn save(&self, batch: &SettlementBatch) -> Result<(), PlatformError> {
        let active_model: settlement_batch::ActiveModel = batch.clone().into();

        let existing = settlement_batch::Entity::find_by_id(batch.settlement_batch_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<SettlementBatch>, PlatformError> {
        let model = settlement_batch::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn find_by_checksum(&self, checksum: &str) -> Result<Option<SettlementBatch>, PlatformError> {
        let model = settlement_batch::Entity::find()
            .filter(settlement_batch::Column::FileChecksum.eq(checksum))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }
}

pub struct PostgresLedgerRepository {
    db: DatabaseConnection,
}

impl PostgresLedgerRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LedgerRepository for PostgresLedgerRepository {
    async fn save(&self, entry: &LedgerEntry) -> Result<(), PlatformError> {
        let active_model: ledger_entry::ActiveModel = entry.clone().into();

        active_model
            .insert(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(())
    }

    async fn find_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<LedgerEntry>, PlatformError> {
        let models = ledger_entry::Entity::find()
            .filter(ledger_entry::Column::TransactionId.eq(transaction_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }

    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, PlatformError> {
        let entries = self.find_by_transaction(transaction_id).await?;

        let total_debit: i64 = entries.iter().map(|e| e.debit_amount_minor_units).sum();
        let total_credit: i64 = entries.iter().map(|e| e.credit_amount_minor_units).sum();

        Ok(total_debit == total_credit)
    }
}
