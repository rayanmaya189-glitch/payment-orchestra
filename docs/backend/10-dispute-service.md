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

```rust
pub struct SubmitRepresentmentCommand {
    pub chargeback_id: Uuid,
    pub evidence: RepresentmentEvidence,
}

// Gap: representment evidence requirements
pub struct RepresentmentEvidence {
    pub transaction_receipt: Option<Uuid>,       // document-service reference
    pub delivery_confirmation: Option<Uuid>,     // proof of delivery
    pub customer_communication: Option<Uuid>,    // email/chat records
    pub cardholder_agreement: Option<Uuid>,      // signed agreement
    pub refund_policy: Option<Uuid>,             // merchant's refund policy
    pub description: String,                     // merchant's description of transaction
    pub supporting_documents: Vec<Uuid>,         // additional evidence documents
}
```

**Preconditions**:
- Chargeback case in `received` or `under_review` status (INV-08)
- Evidence package must include at least transaction receipt
- Representment deadline not passed (scheme-specific: Visa 30 days, Mastercard 45 days)

**Produces**: `RepresentmentSubmitted` (EVT-18)

**Gap: Representment Deadline Tracking**:
- Visa: 30 days from chargeback receipt
- Mastercard: 45 days from chargeback receipt
- mada: per SAMA guidelines (configurable)
- Background job monitors approaching deadlines, alerts merchant via notification-service

### ResolveChargeback

```rust
pub struct ResolveChargebackCommand {
    pub chargeback_id: Uuid,
    pub outcome: ChargebackOutcome,
    pub resolution_note: Option<String>,
}

pub enum ChargebackOutcome {
    Won,        // funds returned to merchant
    Lost,       // chargeback stands
    Accepted,   // merchant accepts the chargeback
    Escalated,  // escalated to scheme arbitration
}
```

**Produces**: `ChargebackResolved` (EVT-19)

**Gap: Chargeback Settlement Impact**:
- When chargeback is `Won`: no settlement adjustment needed (acquirer reverses)
- When chargeback is `Lost`: settlement adjustment recorded as negative `LedgerEntry`
- When chargeback is `Accepted`: merchant voluntarily accepts, no settlement adjustment
- All outcomes update `SettlementExpectation` status accordingly

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

// Gap: representment evidence validation tests
#[tokio::test]
async fn test_submit_representment_requires_receipt() {
    let result = handler.handle(SubmitRepresentmentCommand {
        chargeback_id,
        evidence: RepresentmentEvidence {
            transaction_receipt: None, // missing required evidence
            ..
        },
    }).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}

#[tokio::test]
async fn test_representment_deadline_enforced() {
    // Chargeback received 31 days ago (Visa deadline: 30 days)
    let result = handler.handle(SubmitRepresentmentCommand { ... }).await;
    assert!(matches!(result, Err(PlatformError::Conflict(_))));
}
```

---

## 6. Gap: Chargeback Notification Flow

When a chargeback is received, the platform notifies the merchant immediately:

```
1. Acquirer sends chargeback webhook → connector-gateway
2. connector-gateway normalizes → publishes ChargebackWebhookReceived to NATS
3. dispute-service consumes → RecordChargeback command
4. Chargeback case created with status = 'received'
5. notification-service sends email/SMS to merchant:
   - "New chargeback received for payment {payment_intent_id}"
   - Amount, reason code, deadline for representment
   - Link to dispute dashboard
6. Background job tracks representment deadline
7. If deadline approaching (7 days before): send reminder notification
8. If deadline passed without representment: transition to 'accepted' automatically
```

## 7. Gap: Chargeback-Settlement Interaction

```
1. ChargebackReceived → update settlement expectations
   a. Find SettlementExpectation for the payment_intent_id
   b. If status = 'settled': create adjustment LedgerEntry (debit for chargeback amount)
   c. Update settlement total

2. ChargebackResolved
   a. If Won: no settlement adjustment (acquirer reverses on next settlement)
   b. If Lost: create negative LedgerEntry, update merchant balance
   c. If Accepted: create negative LedgerEntry, update merchant balance
   d. Emit appropriate events for analytics
```
