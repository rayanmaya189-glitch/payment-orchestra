# 06 — invoice-service (BC-06 Invoice Service)

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
CRUD + events. References PaymentIntent by ID (never duplicates payment state).

---

## 1. Domain Model

### AGG-Invoice (Aggregate Root)

**Identity**: `invoice_id: Uuid`

**Entities**: `InvoiceLineItem`

**Invariant**: Uniqueness on `(operator_id, order_reference)` — no duplicate invoices for same order (INV-INV-01).

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "invoice")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub order_reference: String,
    pub status: String, // 'draft' | 'sent' | 'paid' | 'partially_paid' | 'overdue' | 'cancelled'
    pub total_amount_minor_units: i64,
    pub paid_amount_minor_units: i64,
    pub currency: String,
    pub due_date: DateTimeWithTimeZone,
    pub recipient_email: Option<String>,
    pub payment_intent_ids: Vec<Uuid>, // references to BC-05
    pub created_at: DateTimeWithTimeZone,
}
```

---

## 2. Commands

### CreateInvoice

```rust
pub struct CreateInvoiceCommand {
    pub operator_id: Uuid,
    pub order_reference: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub due_date: DateTimeWithTimeZone,
    pub recipient_email: Option<String>,
}
```

**Preconditions**:
- No existing non-Cancelled invoice for same `order_reference` (INV-INV-01)

**Produces**: `InvoiceCreated`

### SendInvoice

**Produces**: `InvoiceSent`

### CancelInvoice

**Produces**: `InvoiceCancelled`

---

## 3. Repository Interface

```rust
#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn load(&self, id: InvoiceId) -> Result<Option<Invoice>, PlatformError>;
    async fn save(&self, invoice: &Invoice) -> Result<(), PlatformError>;
    async fn find_by_order_reference(&self, operator_id: Uuid, order_ref: &str) -> Result<Option<Invoice>, PlatformError>;
    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<Invoice>, PlatformError>;
    async fn find_overdue(&self, operator_id: Uuid) -> Result<Vec<Invoice>, PlatformError>;
}
```

---

## 4. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `INVOICE_NOT_FOUND` | 404 | NOT_FOUND | Invoice does not exist |
| `DUPLICATE_ORDER_INVOICE` | 409 | ALREADY_EXISTS | Invoice already exists for order reference |
| `INVALID_INVOICE_AMOUNT` | 400 | INVALID_ARGUMENT | Invoice total must be positive |
| `INVOICE_ALREADY_SENT` | 409 | FAILED_PRECONDITION | Invoice already sent |
| `INVOICE_ALREADY_CANCELLED` | 409 | FAILED_PRECONDITION | Invoice already cancelled |
| `INVOICE_NOT_PAID` | 409 | FAILED_PRECONDITION | Cannot refund non-paid invoice |

---

## 5. Event Consumer: PaymentCaptured

```rust
// On receiving EVT-04 PaymentCaptured from BC-05:
// 1. Find Invoice linked to this payment_intent_id
// 2. Update paid_amount
// 3. If paid_amount >= total: transition to 'Paid'
// 4. Emit InvoicePaid event
```

---

## 4. TDD Tests

```rust
#[tokio::test]
async fn test_create_invoice_success() {
    let result = handler.handle(CreateInvoiceCommand {
        operator_id,
        order_reference: "ORD-001".into(),
        line_items: vec![InvoiceLineItem { description: "Widget".into(), amount: 10000 }],
        due_date: future_date(),
        recipient_email: Some("customer@example.com".into()),
    }).await.unwrap();
    assert_eq!(result.status, "draft");
}

#[tokio::test]
async fn test_create_duplicate_order_invoice_rejected() {
    handler.handle(CreateInvoiceCommand { order_reference: "ORD-001".into(), ... }).await.unwrap();
    let result = handler.handle(CreateInvoiceCommand { order_reference: "ORD-001".into(), ... }).await;
    assert!(matches!(result, Err(PlatformError::Conflict(ConflictError::DuplicateOrderInvoice))));
}

#[tokio::test]
async fn test_invoice_paid_on_payment_captured() {
    // Create invoice, create payment intent, capture payment
    // Verify invoice status transitions to 'Paid'
}

#[tokio::test]
async fn test_invoice_overdue_transition() {
    // Create invoice with due_date in the past
    // JOB-004 should transition to 'Overdue'
}
```
