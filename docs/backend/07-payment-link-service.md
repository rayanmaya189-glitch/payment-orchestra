# 07 — payment-link-service (BC-07 Payment Link Service)

Public-facing hosted checkout pages. Thin layer over BC-05.

---

## 1. Domain Model

### AGG-PaymentLink (Aggregate Root)

**Identity**: `payment_link_id: Uuid`

**Token**: `plink_{random_32_bytes_base62}` — 128-bit minimum entropy (PLINK-SEC-001)

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_link")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub payment_link_id: Uuid,
    pub operator_id: Uuid,
    pub token: String,             // cryptographically random
    pub status: String,            // 'active' | 'expired' | 'used'
    pub amount_minor_units: i64,
    pub currency: String,
    pub description: Option<String>,
    pub invoice_id: Option<Uuid>,  // optional link to invoice
    pub expires_at: DateTimeWithTimeZone,
    pub used_at: Option<DateTimeWithTimeZone>,
    pub payment_intent_id: Option<Uuid>, // set after checkout
    pub created_at: DateTimeWithTimeZone,
}
```

---

## 2. Commands

### CreatePaymentLink

```rust
pub struct CreatePaymentLinkCommand {
    pub amount: Money,
    pub description: Option<String>,
    pub invoice_id: Option<Uuid>,
    pub expires_in_days: Option<u32>, // default: 30
}
```

**Produces**: `PaymentLinkCreated`

### Hosted Checkout Page

When `GET /pay/{token}` is accessed:
1. Validate token exists and is `active`
2. Check expiry (PLINK-001): expired → HTTP 410 Gone
3. Display minimal checkout page (PLINK-SEC-002: no admin context)
4. On form submit → create PaymentIntent via orchestration-service
5. Redirect to acquirer's 3DS page if required
6. On completion → redirect to success/cancel URL

---

## 3. TDD Tests

```rust
#[tokio::test]
async fn test_create_payment_link() {
    let result = handler.handle(CreatePaymentLinkCommand {
        amount: Money { amount_minor_units: 5000, currency: AED },
        description: Some("Order #123".into()),
        invoice_id: None,
        expires_in_days: Some(30),
    }).await.unwrap();
    assert!(result.token.starts_with("plink_"));
    assert_eq!(result.status, "active");
}

#[tokio::test]
async fn test_expired_payment_link_returns_410() {
    // Create link, advance time past expiry
    let result = handler.get_checkout_page(&token).await;
    assert_eq!(result.status, 410);
}

#[tokio::test]
async fn test_payment_link_token_has_minimum_entropy() {
    // Token must be at least 128 bits of randomness
    // Verify by checking length: base62 of 16 bytes = ~22 chars
    assert!(token.len() >= 22);
}
```
