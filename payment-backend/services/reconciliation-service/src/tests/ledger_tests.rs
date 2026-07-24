//! Ledger balance tests.

use uuid::Uuid;
use chrono::Utc;

use crate::domain::*;
use crate::repository::*;

#[tokio::test]
async fn test_ledger_debit_credit_balanced() {
    let repo = InMemoryReconciliationRepository::new();
    let txn_id = Uuid::now_v7();

    // Add debit and credit entries
    repo.append_ledger_entry(&LedgerEntry {
        entry_id: Uuid::now_v7(),
        transaction_id: txn_id,
        entry_type: EntryType::Debit,
        amount_minor: 10000,
        currency: "AED".into(),
        source_acquirer: "mock_acquirer".into(),
        reconciliation_batch_id: None,
        reconciled: false,
        created_at: Utc::now(),
    }).await.unwrap();

    repo.append_ledger_entry(&LedgerEntry {
        entry_id: Uuid::now_v7(),
        transaction_id: txn_id,
        entry_type: EntryType::Credit,
        amount_minor: 10000,
        currency: "AED".into(),
        source_acquirer: "mock_acquirer".into(),
        reconciliation_batch_id: None,
        reconciled: false,
        created_at: Utc::now(),
    }).await.unwrap();

    let balanced = repo.verify_balance(txn_id).await.unwrap();
    assert!(balanced, "Debit and credit should be balanced");
}

#[tokio::test]
async fn test_ledger_imbalanced_detected() {
    let repo = InMemoryReconciliationRepository::new();
    let txn_id = Uuid::now_v7();

    // Add only a debit (no matching credit)
    repo.append_ledger_entry(&LedgerEntry {
        entry_id: Uuid::now_v7(),
        transaction_id: txn_id,
        entry_type: EntryType::Debit,
        amount_minor: 10000,
        currency: "AED".into(),
        source_acquirer: "mock_acquirer".into(),
        reconciliation_batch_id: None,
        reconciled: false,
        created_at: Utc::now(),
    }).await.unwrap();

    let balanced = repo.verify_balance(txn_id).await.unwrap();
    assert!(!balanced, "Debit without matching credit should be imbalanced");
}
