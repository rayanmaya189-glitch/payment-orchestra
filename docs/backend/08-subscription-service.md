# 08 — subscription-service (BC-08 Subscription Billing)

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
Event-sourced. Owns Subscription aggregate. Handles renewal, dunning, pause/resume.

---

## 1. Domain Model

### AGG-Subscription (Aggregate Root)

**Identity**: `subscription_id: Uuid`

**Entities**: `BillingCycle`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "subscription")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub customer_id: Uuid,
    pub plan_id: String,
    pub status: String, // 'active' | 'past_due' | 'cancelled' | 'paused'
    pub current_period_start: DateTimeWithTimeZone,
    pub current_period_end: DateTimeWithTimeZone,
    pub payment_method_token_id: Uuid,
    pub dunning_retry_count: i32,
    pub max_dunning_retries: i32, // default: 3
    pub created_at: DateTimeWithTimeZone,
    pub cancelled_at: Option<DateTimeWithTimeZone>,
}
```

---

## 2. Invariants

- **INV-SUB-01**: Cannot transition to `Cancelled` while renewal saga is `running` for this subscription
- **JOB-011**: Renewal idempotency key: `{subscription_id}:{billing_cycle_id}`

---

## 3. Commands

### CreateSubscription

**Produces**: `SubscriptionCreated`

### CancelSubscription

**Preconditions**:
- No running renewal saga (INV-SUB-01)

**Produces**: `SubscriptionCancelled`

### PauseSubscription / ResumeSubscription

**Produces**: `SubscriptionPaused` / `SubscriptionResumed`

---

## 4. Repository Interface

```rust
#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn load(&self, id: SubscriptionId) -> Result<Option<Subscription>, PlatformError>;
    async fn save(&self, subscription: &Subscription) -> Result<(), PlatformError>;
    async fn find_active_for_renewal(&self, operator_id: Uuid) -> Result<Vec<Subscription>, PlatformError>;
    async fn find_by_customer(&self, customer_id: Uuid) -> Result<Vec<Subscription>, PlatformError>;
}
```

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `SUBSCRIPTION_NOT_FOUND` | 404 | NOT_FOUND | Subscription does not exist |
| `SUBSCRIPTION_ALREADY_CANCELLED` | 409 | FAILED_PRECONDITION | Subscription already cancelled |
| `CANNOT_CANCEL_DURING_RENEWAL` | 409 | FAILED_PRECONDITION | Renewal saga in progress |
| `INVALID_PLAN_AMOUNT` | 400 | INVALID_ARGUMENT | Plan amount must be positive |
| `DUNNING_EXHAUSTED` | 409 | FAILED_PRECONDITION | Max retries reached |

---

## 6. Scheduled Jobs

- **JOB-001**: Subscription renewal trigger (per billing cycle)
- **JOB-002**: Dunning retry execution

---

## 7. TDD Tests

```rust
#[tokio::test]
async fn test_subscription_renewal_idempotent() {
    // Two scheduler triggers for same cycle should produce same PaymentIntent
    let idem_key = format!("{}:{}", subscription_id, billing_cycle_id);
    let r1 = handler.handle(RenewSubscriptionCommand { subscription_id }).await.unwrap();
    let r2 = handler.handle(RenewSubscriptionCommand { subscription_id }).await.unwrap();
    assert_eq!(r1.payment_intent_id, r2.payment_intent_id);
}

#[tokio::test]
async fn test_subscription_cannot_cancel_during_renewal() {
    // Start renewal saga, try to cancel
    let result = handler.handle(CancelSubscriptionCommand { subscription_id }).await;
    assert!(matches!(result, Err(PlatformError::Conflict(_))));
}

#[tokio::test]
async fn test_dunning_retry_schedule() {
    // Failed renewal → retry at day 1, 3, 7
    // After max retries → PastDue → Cancelled
}
```
