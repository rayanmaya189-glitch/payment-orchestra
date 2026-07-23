//! LedgerEntryRepository implementation for InMemoryReconciliationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryReconciliationRepository;
use super::traits::LedgerEntryRepository;

#[async_trait]
impl LedgerEntryRepository for InMemoryReconciliationRepository {
    async fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<(), ReconciliationError> {
        self.ledger.write().await.push(entry.clone());
        Ok(())
    }

    async fn verify_balance(&self, transaction_id: Uuid) -> Result<bool, ReconciliationError> {
        let ledger = self.ledger.read().await;
        let total_debit: i64 = ledger.iter()
            .filter(|e| e.transaction_id == transaction_id && e.entry_type == EntryType::Debit)
            .map(|e| e.amount_minor)
            .sum();
        let total_credit: i64 = ledger.iter()
            .filter(|e| e.transaction_id == transaction_id && e.entry_type == EntryType::Credit)
            .map(|e| e.amount_minor)
            .sum();
        Ok(total_debit == total_credit)
    }

    async fn find_imbalanced(&self) -> Result<Vec<Uuid>, ReconciliationError> {
        let ledger = self.ledger.read().await;
        let mut txn_ids: Vec<Uuid> = ledger.iter().map(|e| e.transaction_id).collect();
        txn_ids.sort();
        txn_ids.dedup();

        let mut imbalanced = Vec::new();
        for txn_id in txn_ids {
            let total_debit: i64 = ledger.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == EntryType::Debit)
                .map(|e| e.amount_minor)
                .sum();
            let total_credit: i64 = ledger.iter()
                .filter(|e| e.transaction_id == txn_id && e.entry_type == EntryType::Credit)
                .map(|e| e.amount_minor)
                .sum();
            if total_debit != total_credit {
                imbalanced.push(txn_id);
            }
        }
        Ok(imbalanced)
    }
}
