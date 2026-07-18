# 14 — notification-service (BC-14 Notification Service)

At-least-once delivery. Subscribes to domain events. Email/SMS dispatch.

---

## 1. Domain Model

### AGG-NotificationRequest (Root)

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_request")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub notification_id: Uuid,
    pub operator_id: Uuid,
    pub notification_type: String, // 'email' | 'sms' | 'webhook'
    pub recipient: String,
    pub template_id: String,
    pub payload_json: String,
    pub status: String, // 'queued' | 'sent' | 'failed' | 'dead_letter'
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: DateTimeWithTimeZone,
    pub sent_at: Option<DateTimeWithTimeZone>,
}
```

---

## 2. Event Subscriptions

| Event | Notification Type | Template |
|---|---|---|
| `PaymentFailedAllRoutes` | Email to operator | "Payment failed on all acquirers" |
| `SettlementRecordUnmatched` | Email to finance | "Unmatched settlement record detected" |
| `ChargebackReceived` | Email to operator + compliance | "New chargeback received" |
| `SubscriptionRenewalFailed` | Email to customer | "Subscription payment failed" |
| `ApiKeyExpiring` | Email to operator | "API key expiring in X days" |

---

## 3. Delivery Dedup

```rust
// Redis key: notif_sent:{notification_id}
// TTL: 7 days
// On send: SET if not exists → if exists, skip (idempotent)
```

---

## 4. TDD Tests

```rust
#[tokio::test]
async fn test_notification_sent_on_payment_failed() {
    // Emit PaymentFailedAllRoutes event
    // Verify notification created and sent
}

#[tokio::test]
async fn test_notification_dedup() {
    // Same notification_id sent twice → only one email
}

#[tokio::test]
async fn test_notification_retry_on_failure() {
    // Mock email provider fails → retry with backoff
}
```
