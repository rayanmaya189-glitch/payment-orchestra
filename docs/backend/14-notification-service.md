# 14 — notification-service (BC-14 Notification Service)

> ⚡ **Pure Router**: The platform is a routing and orchestration layer only. Funds flow directly between the customer, the payment gateway, and the merchant bank account. The platform never holds, touches, or controls funds.

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
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

## 4. Repository Interface

```rust
#[async_trait]
pub trait NotificationRequestRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<NotificationRequest>, PlatformError>;
    async fn save(&self, request: &NotificationRequest) -> Result<(), PlatformError>;
    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, PlatformError>;
    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, PlatformError>;
}
```

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `NOTIFICATION_NOT_FOUND` | 404 | NOT_FOUND | Notification does not exist |
| `NOTIFICATION_TEMPLATE_MISSING` | 500 | INTERNAL | Template not found |
| `EMAIL_PROVIDER_UNAVAILABLE` | 503 | UNAVAILABLE | Email provider down |
| `SMS_PROVIDER_UNAVAILABLE` | 503 | UNAVAILABLE | SMS provider down |
| `NOTIFICATION_ALREADY_SENT` | 409 | FAILED_PRECONDITION | Duplicate notification (dedup) |

---

## 6. TDD Tests

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


