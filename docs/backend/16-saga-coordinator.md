# 16 — Saga Coordinator (BC-17)

Cross-cutting infrastructure. Durable state machine for multi-step, cross-aggregate workflows.

---

## 1. Domain Model

### AGG-SagaInstance (Root)

**State Machine**:
```
created → running → completed | compensating → compensated | failed → requires_manual_intervention
```

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "saga_instances")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub saga_id: Uuid,
    pub saga_type: String,
    pub aggregate_id: Uuid,
    pub status: String,
    pub current_step: String,
    pub steps_completed: String,      // JSON array
    pub steps_compensated: String,    // JSON array
    pub compensation_attempts: i32,
    pub max_compensation_retries: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deadline_at: Option<DateTimeWithTimeZone>,
}
```

---

## 2. Sagas

| Saga | Trigger | Steps | Compensation |
|---|---|---|---|
| Payment Lifecycle | CreatePaymentIntent | Authorize → Capture → Settle → Reconcile | Void on failure |
| Subscription Renewal | Scheduler | Create renewal intent → Authorize → Handle dunning | Cancel on exhausted retries |
| Reconciliation Resolution | ResolveException | Match → Confirm → Update status | Undo match on failure |
| Invoice Payment | InvoiceSent + payment | Create intent → Authorize → Capture → Update invoice | Void if cancelled |

---

## 3. Compensation Rules

- **SAGA-006**: Reverse-order compensation (stack-based)
- **SAGA-007**: Retry up to 3 times with exponential backoff
- **SAGA-008**: Step timeout detection via background sweep
- **SAGA-009**: Compensation idempotency (check aggregate state before acting)

---

## 4. TDD Tests

```rust
#[tokio::test]
async fn test_payment_lifecycle_saga_success() {
    // CreatePaymentIntent → Authorize → Capture → Complete
    let saga = saga_coordinator.start(PaymentLifecycleSaga { payment_intent_id }).await.unwrap();
    assert_eq!(saga.status, "completed");
}

#[tokio::test]
async fn test_payment_lifecycle_saga_compensates_on_capture_failure() {
    // Authorize succeeds, Capture fails → saga compensates (voids)
    let saga = saga_coordinator.start(PaymentLifecycleSaga { payment_intent_id }).await.unwrap();
    assert_eq!(saga.status, "compensated");
    // Verify PaymentIntent is in Voided state
}

#[tokio::test]
async fn test_compensation_idempotent() {
    // Void already voided → no error, just success
}

#[tokio::test]
async fn test_saga_timeout_triggers_compensation() {
    // Step exceeds timeout → saga transitions to compensating
}
```
