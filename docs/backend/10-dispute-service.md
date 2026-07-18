# 10 — dispute-service (BC-10 Dispute Management)

Event-sourced. Owns ChargebackCase aggregate.

---

## 1. Domain Model

### AGG-ChargebackCase (Aggregate Root)

**Identity**: `chargeback_id: Uuid`

**Entities**: `RepresentmentSubmission`

**Value Objects**: `ChargebackReasonCode`, `ChargebackOutcome`

**Invariant (INV-08)**: Must reference exactly one Captured PaymentIntent.

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "chargeback_case")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub chargeback_id: Uuid,
    pub operator_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub status: String, // 'received' | 'under_review' | 'representment_submitted' | 'won' | 'lost' | 'accepted'
    pub reason_code: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub received_at: DateTimeWithTimeZone,
    pub resolved_at: Option<DateTimeWithTimeZone>,
    pub outcome: Option<String>,
}
```

---

## 2. Commands

### RecordChargeback

```rust
pub struct RecordChargebackCommand {
    pub payment_intent_id: Uuid,
    pub reason_code: String,
    pub amount: Money,
    pub acquirer_notification: Vec<u8>,
}
```

**Preconditions**:
- PaymentIntent exists and is in `Captured` state (INV-08)

**Produces**: `ChargebackReceived` (EVT-17)

### SubmitRepresentment

**Produces**: `RepresentmentSubmitted` (EVT-18)

### ResolveChargeback

**Produces**: `ChargebackResolved` (EVT-19)

---

## 3. Repository Interface

```rust
#[async_trait]
pub trait ChargebackCaseRepository: Send + Sync {
    async fn load(&self, id: ChargebackId) -> Result<Option<ChargebackCase>, PlatformError>;
    async fn save(&self, case: &ChargebackCase) -> Result<(), PlatformError>;
    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<ChargebackCase>, PlatformError>;
    async fn find_open_cases(&self, operator_id: Uuid) -> Result<Vec<ChargebackCase>, PlatformError>;
}
```

---

## 4. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `CHARGEBACK_NOT_FOUND` | 404 | NOT_FOUND | Chargeback case does not exist |
| `PAYMENT_INTENT_NOT_CAPTURED` | 409 | FAILED_PRECONDITION | Cannot create chargeback for non-captured intent |
| `CHARGEBACK_ALREADY_RESOLVED` | 409 | FAILED_PRECONDITION | Chargeback already resolved |
| `CHARGEBACK_ALREADY_DISPUTED` | 409 | FAILED_PRECONDITION | Representment already submitted |
| `INVALID_REPRESENTMENT_EVIDENCE` | 400 | INVALID_ARGUMENT | Missing required evidence fields |

---

## 5. TDD Tests

```rust
#[tokio::test]
async fn test_record_chargeback_success() {
    let result = handler.handle(RecordChargebackCommand {
        payment_intent_id: captured_intent_id,
        reason_code: "fraud".into(),
        amount: Money { amount_minor_units: 5000, currency: AED },
        acquirer_notification: vec![],
    }).await.unwrap();
    assert_eq!(result.status, "received");
}

#[tokio::test]
async fn test_record_chargeback_on_uncaptured_intent_rejected() {
    let result = handler.handle(RecordChargebackCommand {
        payment_intent_id: authorized_intent_id, // not captured
        ...
    }).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}
```
