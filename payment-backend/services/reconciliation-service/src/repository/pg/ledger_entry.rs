use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresReconciliationRepository;
use crate::repository::LedgerEntryRepository;
use crate::domain::*;
use crate::entities::ledger_entry::{
    Entity as LedgerEntryEntity,
    ActiveModel as LedgerEntryActiveModel,
    Model as LedgerEntryModel,
    Column as LedgerEntryColumn,
};

#[async_trait]
impl LedgerEntryRepository for PostgresReconciliationRepository {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError> {
        let model = ledger_entry_domain_to_model(entry);
        LedgerEntryEntity::insert(LedgerEntryActiveModel::from(model))
            .exec(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError> {
        let entries = LedgerEntryEntity::find()
            .filter(LedgerEntryColumn::TransactionId.eq(transaction_id))
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;

        let total_debit: i64 = entries.iter()
            .filter(|e| e.entry_type == "Debit")
            .map(|e| e.amount_minor)
            .sum();

        let total_credit: i64 = entries.iter()
            .filter(|e| e.entry_type == "Credit")
            .map(|e| e.amount_minor)
            .sum();

        Ok(total_debit == total_credit)
    }

    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError> {
        let entries = LedgerEntryEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;

        let mut txn_ids: Vec<Uuid> = entries.iter().map(|e| e.transaction_id).collect();
        txn_ids.sort();
        txn_ids.dedup();

        let mut imbalanced = Vec::new();
        for txn_id in txn_ids {
            let total_debit: i64 = entries.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == "Debit")
                .map(|e| e.amount_minor)
                .sum();
            let total_credit: i64 = entries.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == "Credit")
                .map(|e| e.amount_minor)
                .sum();
            if total_debit != total_credit {
                imbalanced.push(txn_id);
            }
        }
        Ok(imbalanced)
    }
}

fn ledger_entry_domain_to_model(entry: &LedgerEntry) -> LedgerEntryModel {
    LedgerEntryModel {
        entry_id: entry.entry_id,
        transaction_id: entry.transaction_id,
        entry_type: match entry.entry_type {
            EntryType::Debit => "Debit".to_string(),
            EntryType::Credit => "Credit".to_string(),
        },
        amount_minor: entry.amount_minor,
        currency: entry.currency.clone(),
        created_at: entry.created_at,
    }
}
