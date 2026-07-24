//! LedgerEntry append-only aggregate (AGG-02).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::EntryType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: EntryType,
    pub amount_minor: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub reconciliation_batch_id: Option<Uuid>,
    pub reconciled: bool,
    pub created_at: DateTime<Utc>,
}
