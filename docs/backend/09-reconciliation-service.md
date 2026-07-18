# 09 — reconciliation-service (BC-09 Settlement & Reconciliation)

Event-sourced. Owns SettlementBatch, LedgerEntry aggregates. Matches acquirer settlements against PaymentIntents.

---

## 1. Domain Model

### AGG-SettlementBatch (Aggregate Root)

**Identity**: `settlement_batch_id: Uuid`

**Entities**: `SettlementRecord` (child)

**Value Objects**: `SettlementFileChecksum`, `SettlementMatchOutcome`, `FeeBreakdown`

**Invariant (INV-06)**: Batch cannot be re-ingested if checksum matches previous batch.

**Invariant (INV-07)**: Matched record references exactly one PaymentIntent; amount recorded even if different from captured amount.

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "settlement_batch")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub settlement_batch_id: Uuid,
    pub operator_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub file_checksum: String,      // SHA-256
    pub file_format: String,
    pub status: String,             // 'ingesting' | 'processed' | 'quarantined'
    pub total_records: i32,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub total_amount_minor_units: i64,
    pub ingested_at: DateTimeWithTimeZone,
    pub processed_at: Option<DateTimeWithTimeZone>,
}
```

### AGG-LedgerEntry (Append-Only)

**Identity**: `entry_id: Uuid`

**Invariant (INV-10)**: SUM(debit) - SUM(credit) = 0 per transaction_id.

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ledger_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub entry_id: Uuid,
    pub transaction_id: Uuid,
    pub entry_type: String,
    pub debit_amount_minor_units: i64,
    pub credit_amount_minor_units: i64,
    pub currency: String,
    pub source_acquirer: String,
    pub reconciliation_batch_id: Option<Uuid>,
    pub reconciled: bool,
    pub created_at: DateTimeWithTimeZone,
}
```

---

## 2. Reconciliation Matching Algorithm

```rust
pub enum MatchStrategy {
    Exact { confidence: f64 },           // 100%
    Fuzzy { confidence_range: (f64, f64) }, // 70-99%
    AiAssisted { confidence_range: (f64, f64) }, // 50-70%
}

pub struct ReconciliationMatcher {
    strategies: Vec<Box<dyn MatchStrategyImpl>>,
    auto_confirm_threshold: f64,    // default: 0.95
    review_threshold: f64,          // default: 0.70
}
```

**Matching Rules**:

| Strategy | Inputs | Confidence | Action |
|---|---|---|---|
| Exact | acquirer_reference → payment_intent_id | 100% | Auto-confirm |
| Amount+Date Fuzzy | amount ± fee tolerance AND date ±2 days | 70-99% | Queue for review |
| AI-Assisted | vector similarity over attributes | 50-70% | Queue for review |
| No Match | — | < 50% | Flag as unmatched |

---

## 3. Commands

### IngestSettlementBatch

```rust
pub struct IngestSettlementBatchCommand {
    pub acquirer_link_id: Uuid,
    pub raw_file: Vec<u8>,
    pub file_format: SettlementFormat,
}
```

**Preconditions**:
- Acquirer link is `Active`
- File checksum doesn't match previously ingested batch (INV-006)

**Produces**: `SettlementBatchIngested` (EVT-13), then `SettlementRecordMatched`/`SettlementRecordUnmatched` per record

**TDD Tests**:

```rust
#[tokio::test]
async fn test_ingest_settlement_batch_success() {
    let result = handler.handle(IngestSettlementBatchCommand {
        acquirer_link_id,
        raw_file: settlement_file_bytes,
        file_format: SettlementFormat::Webhook,
    }).await.unwrap();
    assert_eq!(result.status, "processed");
    assert!(result.matched_count > 0);
}

#[tokio::test]
async fn test_ingest_duplicate_batch_rejected() {
    // Same checksum as previously ingested
    handler.handle(IngestSettlementBatchCommand { ... }).await.unwrap();
    let result = handler.handle(IngestSettlementBatchCommand { ... }).await;
    assert!(matches!(result, Err(PlatformError::Conflict(_))));
}

#[tokio::test]
async fn test_exact_match_produces_auto_confirm() {
    // Settlement record with matching acquirer_reference
    let result = matcher.match_record(&settlement_record, &payment_intents).unwrap();
    assert_eq!(result.confidence, 1.0);
    assert!(result.auto_confirm);
}

#[tokio::test]
async fn test_unmatched_record_flagged() {
    // Settlement record with no matching acquirer_reference
    let result = matcher.match_record(&settlement_record, &payment_intents).unwrap();
    assert_eq!(result.outcome, SettlementMatchOutcome::Unmatched);
}

#[tokio::test]
async fn test_amount_mismatch_record_flagged() {
    // Settlement record with different amount than captured
    let result = matcher.match_record(&settlement_record, &payment_intents).unwrap();
    assert_eq!(result.outcome, SettlementMatchOutcome::AmountMismatch);
}
```

---

## 4. Domain Events

| Event | Consumer |
|---|---|
| `SettlementBatchIngested` | analytics-service |
| `SettlementRecordMatched` | invoice-service, subscription-service, analytics-service |
| `SettlementRecordUnmatched` | notification-service (alert), ai-assistant-service |
| `ReconciliationExceptionResolved` | analytics-service |
| `LedgerEntryCreated` | (internal) |
| `LedgerEntryReconciled` | analytics-service |

---

## 5. Background Jobs

- **JOB-003**: Settlement file polling for SFTP connectors
- **JOB-006**: Reconciliation exception aging alerts
- **LEDGER-VERIFY-001**: Daily ledger balance verification
- **CONSIST-001**: Daily cross-service consistency check (invoice ↔ payment)
