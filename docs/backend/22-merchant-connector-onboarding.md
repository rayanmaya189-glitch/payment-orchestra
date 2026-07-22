# 22 — Merchant Connector Onboarding (BYOK Integration Flow)

**Cross-service flow specification.**  
Describes the end-to-end flow for a merchant to connect their own payment gateway credentials (BYOK) and configure routing.

---

## 1. Overview

The BYOK onboarding flow spans four services:

| Service | Role |
|---|---|
| `api-gateway` | Merchant-facing REST+protobuf endpoints for connector management |
| `connector-gateway` | Provides `OnboardingSchema` per connector; validates credentials |
| `merchant-acquirer-link-service` | Owns `MerchantAcquirerLink` lifecycle; encrypts credentials |
| `orchestration-service` | Consumes link state changes for routing |

---

## 2. Flow: List Available Connectors

**Endpoint: POST /v1/connectors (protobuf body with optional filters, returns list of available connectors)
**Auth:** Bearer JWT (merchant admin)

**Response:**

```json
{
  "connectors": [
    {
      "id": "network_international",
      "display_name": "Network International",
      "description": "UAE's leading acquirer — best for local card processing",
      "supported_environments": ["sandbox", "production"],
      "supported_card_schemes": ["visa", "mastercard"],
      "supported_currencies": ["AED"],
      "settlement_format": "webhook",
      "settlement_cycle": "T+1",
      "docs_url": "https://docs.networkinternational.com/api"
    },
    {
      "id": "checkout_com",
      "display_name": "Checkout.com",
      "description": "Global PSP — multi-currency, strong fraud tools",
      "supported_environments": ["sandbox", "production"],
      "supported_card_schemes": ["visa", "mastercard", "amex"],
      "supported_currencies": ["AED", "USD", "EUR", "GBP"],
      "settlement_format": "webhook",
      "settlement_cycle": "T+2",
      "docs_url": "https://docs.checkout.com"
    }
  ]
}
```

---

## 3. Flow: Get Connector Schema

**Endpoint: POST /v1/connectors/{connector_id}/schema (protobuf body: empty, returns OnboardingSchema protobuf)
**Auth:** Bearer JWT (merchant admin)

Returns the credential fields the merchant needs to fill in. The frontend renders a dynamic form from this schema.

**Response (Checkout.com):**

```json
{
  "connector_id": "checkout_com",
  "fields": [
    {
      "name": "secret_key",
      "type": "password",
      "required": true,
      "label": "Secret Key",
      "placeholder": "sk_live_...",
      "validation": { "regex": "^sk_(test|live)_[a-zA-Z0-9]+$", "min_length": 32, "max_length": 128 },
      "help_text": "Find this in your Checkout.com dashboard under Settings > API Keys",
      "docs_url": "https://docs.checkout.com/docs/api-keys"
    },
    {
      "name": "public_key",
      "type": "password",
      "required": true,
      "label": "Public Key",
      "placeholder": "pk_live_...",
      "validation": { "regex": "^pk_(test|live)_[a-zA-Z0-9]+$", "min_length": 24, "max_length": 64 },
      "help_text": "Your public key from the same API Keys section",
      "docs_url": "https://docs.checkout.com/docs/api-keys"
    },
    {
      "name": "environment",
      "type": "select",
      "required": true,
      "label": "Environment",
      "options": [
        { "value": "sandbox", "label": "Sandbox (Testing)" },
        { "value": "production", "label": "Production (Live)" }
      ],
      "help_text": "Use Sandbox for testing. Switch to Production when ready to accept live payments."
    },
    {
      "name": "webhook_secret",
      "type": "password",
      "required": false,
      "label": "Webhook Secret Key",
      "placeholder": "whsec_...",
      "validation": { "regex": "^whsec_[a-zA-Z0-9]+$", "min_length": 16 },
      "help_text": "Optional. Used to verify webhook signatures. Set up in Dashboard > Webhooks."
    }
  ],
  "test_card_numbers": [
    { "brand": "Visa", "number": "4242424242424242", "type": "success" },
    { "brand": "Visa", "number": "4000000000000002", "type": "decline" },
    { "brand": "Mastercard", "number": "5555555555554444", "type": "success" },
    { "brand": "Amex", "number": "378282246310005", "type": "success" },
    { "brand": "Mada", "number": "5432101234567890", "type": "success" }
  ]
}
```

---

## 4. Flow: Create MerchantAcquirerLink (Connect Gateway)

**Endpoint:** `POST /v1/merchant-links`  
**Auth:** Bearer JWT (merchant admin)

**Request:**

```json
{
  "connector_id": "checkout_com",
  "display_name": "Production Checkout.com Gateway",
  "environment": "production",
  "credentials": {
    "secret_key": "sk_live_abc123def456",
    "public_key": "pk_live_xyz789",
    "environment": "production"
  }
}
```

**Backend Flow:**

```
1. Validate request against connector's OnboardingSchema
2. Call connector-gateway: validate_credentials(credentials)
   → Sandbox test: GET /payment-status endpoint
3. If validation fails → return 400 CREDENTIALS_INVALID
4. Encrypt credentials via KMS (envelope encryption)
5. Store MerchantAcquirerLink (status = "testing")
6. Emit MerchantAcquirerLinkCreated event
7. Return response with test_connection_url
```

**Response:**

```json
{
  "link_id": "link_abc123",
  "connector_id": "checkout_com",
  "display_name": "Production Checkout.com Gateway",
  "environment": "production",
  "status": "testing",
  "health_status": "unknown",
  "created_at": "2026-07-22T10:30:00Z",
  "test_connection_url": "/v1/merchant-links/link_abc123/test"
}
```

---

## 5. Flow: Test Connection

**Endpoint:** `POST /v1/merchant-links/{link_id}/test`  
**Auth:** Bearer JWT (merchant admin)

**Request:** (empty body)

**Backend Flow:**

```
1. Load link, decrypt credentials
2. Call connector-gateway: test_connection(decrypted_credentials)
3. Test results:
   - Success: status → 'active', health → 'healthy'
   - Failure: status stays 'testing', return detailed error
4. Emit MerchantAcquirerConnectionTested event
5. If success → merchant can now configure routing
```

**Response (Success):**

```json
{
  "link_id": "link_abc123",
  "status": "active",
  "health_status": "healthy",
  "test_result": {
    "success": true,
    "latency_ms": 245,
    "merchant_name": "Acme Corp - NI Account",
    "permissions": ["authorize", "capture", "refund", "void"]
  }
}
```

**Response (Failure):**

```json
{
  "link_id": "link_abc123",
  "status": "testing",
  "test_result": {
    "success": false,
    "error": "CREDENTIALS_INVALID",
    "error_message": "Authentication failed: Invalid API Key. Verify your key at https://dashboard.networkinternational.com/settings/api-keys",
    "latency_ms": 1230
  }
}
```

---

## 6. Flow: Configure Routing

After connecting at least one gateway, the merchant configures routing:

**Endpoint:** `POST /v1/routing-policies`  
**Auth:** Bearer JWT (merchant admin)

**Request:**

```json
{
  "name": "Primary Routing Policy",
  "rules": [
    {
      "priority": 1,
      "link_id": "link_abc123",
      "conditions": {
        "card_schemes": ["visa", "mastercard"],
        "currencies": ["AED"],
        "min_amount": { "value": 100, "currency": "AED" },
        "max_amount": { "value": 50000000, "currency": "AED" }
      }
    },
    {
      "priority": 2,
      "link_id": "link_def456",
      "conditions": {
        "card_schemes": ["visa", "mastercard", "amex"],
        "currencies": ["USD", "EUR", "GBP"],
        "max_amount": { "value": 100000000, "currency": "USD" }
      }
    }
  ],
  "failover": {
    "max_hops": 3,
    "retry_decline_codes": ["insufficient_funds", "do_not_honor", "issuer_unavailable"],
    "latency_budget_ms": 10000
  },
  "partial_authorization": "retry_next_acquirer"
}
```

---

## 7. Flow: View Connected Gateways

**Endpoint: POST /v1/merchant-links/search (protobuf body with optional filters)
**Auth:** Bearer JWT (merchant admin)

**Response:**

```json
{
  "links": [
    {
      "link_id": "link_abc123",
      "connector": {
        "id": "network_international",
        "display_name": "Network International"
      },
      "display_name": "NI Production - Main",
      "environment": "production",
      "status": "active",
      "health_status": "healthy",
      "last_tested_at": "2026-07-22T10:35:00Z",
      "created_at": "2026-07-20T08:00:00Z",
      "routing_priority": 1,
      "today_volume": { "value": 1250000, "currency": "AED" },
      "daily_limit": { "value": 50000000, "currency": "AED" }
    },
    {
      "link_id": "link_def456",
      "connector": {
        "id": "checkout_com",
        "display_name": "Checkout.com"
      },
      "display_name": "CKO - Multi Currency",
      "environment": "production",
      "status": "active",
      "health_status": "healthy",
      "last_tested_at": "2026-07-22T09:00:00Z",
      "created_at": "2026-07-19T14:00:00Z",
      "routing_priority": 2,
      "today_volume": { "value": 450000, "currency": "AED" },
      "daily_limit": { "value": 100000000, "currency": "AED" }
    }
  ]
}
```

---

## 8. BYOK Dashboard Pages (Frontend Spec)

### 8.1 Gateway Configuration List

- Table of connected gateways with status indicators (green/red/yellow dot)
- Quick actions: Test, Disable, Configure, View Health
- Add Gateway button
- Search/filter by connector type, status, environment
- Volume and health metrics per gateway

### 8.2 Add/Edit Gateway Form

- Dynamic form rendered from connector's OnboardingSchema
- Field-level validation (regex, required, length)
- Environment selector (sandbox/production)
- Credential input with show/hide toggle
- Help text and documentation links per field
- "Test & Save" button (validates before saving)
- Test card numbers section for sandbox testing

### 8.3 Gateway Detail Page

- Connection health timeline (uptime, latency chart)
- Credential expiry countdown
- Transaction volume chart (today, 7 days, 30 days)
- Success rate trend
- Fee breakdown (interchange, scheme, acquirer fees)
- Routing rules using this gateway
- Activity log (connection tests, credential rotations, status changes)

### 8.4 Routing Policy Editor

- Visual drag-and-drop rule ordering
- Rule conditions editor (card scheme, currency, amount range)
- Failover config panel
- A/B test configuration (percentage-based traffic split)
- Policy activation with Maker/Checker approval

---

## 9. Credential Health Monitoring Dashboards

### 9.1 Merchant-Facing Dashboard Widget

```
┌─────────────────────────────────────────────────────────────┐
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │  NI Production   │  │  Checkout Prod    │                │
│  │  ● Active        │  │  ● Active         │                │
│  │  2,350 tx today  │  │  890 tx today     │                │
│  │  98.2% success   │  │  97.5% success    │                │
│  │  180ms avg       │  │  210ms avg        │                │
│  └──────────────────┘  └──────────────────┘                │
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  Recent Health Events                                   ││
│  │  ✓ 10:30 — NI Production — Connection test passed      ││
│  │  ⚠ 09:15 — CKO Sandbox — Credentials expiring in 7d    ││
│  │  ✓ 08:00 — NI Production — Connection test passed      ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### 9.2 Platform-Wide Operator Health (Admin)

```
┌─────────────────────────────────────────────────────────────┐
│  Operator                  Gateways   Status   Volume       │
│  Acme Corp                 3/3 active  ● All    ₿ 2.3M     │
│  TechCo LLC                2/3 active  ⚠ 1 down ₿ 890K     │
│  Retail Inc                1/2 active  ❌ 1 expired ₿ 450K  │
│  Finance Ltd               2/2 active  ● All    ₿ 1.1M     │
└─────────────────────────────────────────────────────────────┘
```

---

## 10. Error Handling & Edge Cases

| Scenario | User Experience | System Behavior |
|---|---|---|
| Invalid credentials | Show field-level error: "Invalid format: Secret Key must start with sk_live_ or sk_test_" | Reject at schema validation |
| Credentials fail sandbox test | Show: "Could not connect to Checkout.com: Authentication failed. Check your Secret Key." | Link stays in `testing` status |
| Gateway API is down | Show: "Network International is temporarily unreachable. Try again in a few minutes." | Schedule retry with backoff |
| Credentials expire | Push notification + email: "Your Checkout.com credentials expire in 14 days" | Auto-disable on expiry, emit event |
| Dual-key rotation conflict | Show: "Both old and new credentials failed. Rolled back to old credentials." | Revert to old credentials |
| Duplicate credentials | Show: "These credentials are already in use for another gateway." | Return reference to existing link |

---

## 11. TDD Tests

```rust
#[tokio::test]
async fn test_full_onboarding_flow() {
    // 1. List connectors → expect non-empty list
    let connectors = onboarding_handler.list_connectors().await.unwrap();
    assert!(!connectors.is_empty());

    // 2. Get schema for checkout_com → expect credential fields
    let schema = onboarding_handler.get_schema("checkout_com").await.unwrap();
    assert!(schema.fields.iter().any(|f| f.name == "secret_key"));

    // 3. Create link with valid credentials → status = testing
    let link = onboarding_handler.create_link(CreateMerchantAcquirerLinkCommand {
        connector_id: "checkout_com".into(),
        credentials: valid_checkout_credentials(),
        ..default()
    }).await.unwrap();
    assert_eq!(link.status, "testing");

    // 4. Test connection → status = active
    let test = onboarding_handler.test_connection(link.link_id).await.unwrap();
    assert!(test.success);
    let link = link_repo.load(link.link_id).await.unwrap();
    assert_eq!(link.status, "active");

    // 5. Create routing policy → active
    let policy = routing_handler.activate(ActivateRoutingPolicyCommand {
        rules: vec![RoutingRule { link_id: link.link_id, priority: 1, ..default() }],
        ..default()
    }).await.unwrap();
    assert_eq!(policy.status, "active");
}

#[tokio::test]
async fn test_onboarding_with_invalid_credentials_rejected() {
    let result = onboarding_handler.create_link(CreateMerchantAcquirerLinkCommand {
        credentials: RawConnectorCredentials {
            fields: vec![("secret_key".into(), "invalid".into())].into_iter().collect(),
        },
        ..default()
    }).await;
    assert!(matches!(result, Err(PlatformError::Validation(_))));
}

#[tokio::test]
async fn test_credential_rotation_keeps_service_running() {
    // Create link → rotate credentials → test still works
    let link = onboarding_handler.create_link(/* ... */).await.unwrap();
    onboarding_handler.rotate_credentials(RotateMerchantAcquirerCredentialsCommand {
        link_id: link.link_id,
        new_credentials: new_checkout_credentials(),
        rotate_immediately: false,
    }).await.unwrap();
    let test = onboarding_handler.test_connection(link.link_id).await.unwrap();
    assert!(test.success);
}

#[tokio::test]
async fn test_auto_disable_on_credential_expiry() {
    // Simulate credential expiry
    let result = onboarding_handler.sweep_expired_credentials().await.unwrap();
    assert_eq!(result.disabled_count, expected_count);
}
```
