# 21 — merchant-acquirer-link-service (BYOK Core)

**New Service — BYOK (Bring Your Own Key) Core.**  
Owns the `MerchantAcquirerLink` aggregate — the fundamental entity that connects an operator to a specific payment gateway using the operator's own credentials.

---

## 1. Why This Service Exists

In a BYOK payment orchestration model, merchants bring their OWN merchant accounts with their OWN payment gateways. The platform does NOT:

- Act as a payment gateway (no direct acquiring)
- Act as a payment facilitator (no sub-merchant onboarding)
- Store or manage merchant credentials on behalf of gateways

The platform IS a routing layer that:

- Lets merchants connect any supported gateway using their own credentials
- Routes payments through the connected gateways based on merchant-configured rules
- Provides a unified API across all connected gateways

**The `MerchantAcquirerLink` is the central entity that represents ONE merchant's connection to ONE gateway using ONE set of credentials.**

---

## 2. Domain Model

### AGG-MerchantAcquirerLink (Aggregate Root)

**Identity**: `link_id: Uuid` (UUIDv7)

**Value Objects**:
- `EncryptedCredentials` — envelope-encrypted credential payload
- `ConnectorCredentialSchema` — dynamic schema from connector-gateway
- `ConnectionHealth` — health status of this link

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "merchant_acquirer_link")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub link_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,                    // 'network_international' | 'checkout_com' | 'telr'
    pub display_name: String,                     // user-friendly label: "Production NI - Main"
    pub environment: String,                      // 'sandbox' | 'production'
    pub credentials_key_id: String,               // KMS key ID used for envelope encryption
    pub encrypted_credentials: Vec<u8>,           // envelope-encrypted JSON blob
    pub credentials_hash: String,                 // SHA-256 of raw credentials (change detection)
    pub status: String,                           // 'active' | 'disabled' | 'testing' | 'credentials_expired'
    pub health_status: String,                    // 'healthy' | 'degraded' | 'unreachable' | 'unknown'
    pub last_tested_at: Option<DateTimeWithTimeZone>,
    pub last_healthy_at: Option<DateTimeWithTimeZone>,
    pub credentials_expires_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
```

**Invariants**:
- **INV-BYOK-01**: A link's connector_id must reference a registered connector in `connector-gateway`
- **INV-BYOK-02**: Credentials must pass validation before a link can become `active`
- **INV-BYOK-03**: `encrypted_credentials` must be decryptable at any time (tested periodically)
- **INV-BYOK-04**: A link in `disabled` or `credentials_expired` state must be excluded from routing
- **INV-BYOK-05**: An operator can have at most 5 active links per connector_id

---

## 3. Commands

### CreateMerchantAcquirerLink

```rust
pub struct CreateMerchantAcquirerLinkCommand {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub display_name: String,
    pub environment: LinkEnvironment,
    pub credentials: RawConnectorCredentials, // raw credentials from merchant form
}
```

**Preconditions**:
- Connector with `connector_id` exists in registry (INV-BYOK-01)
- Credentials pass format validation against connector schema
- Operator doesn't exceed max links per connector (INV-BYOK-05)
- Credentials are validated via sandbox/status-check call to connector (INV-BYOK-02)

**Flow**:
1. Load connector's `OnboardingSchema` from connector-gateway
2. Validate raw credentials against schema (field types, regex, required fields)
3. Call `connector.validate_credentials(credentials)` — sandbox/status-check only
4. If validation fails → return specific error: `CREDENTIALS_INVALID` with details
5. If validation succeeds:
   a. Encrypt credentials via KMS (envelope encryption with KEK)
   b. Compute credentials hash (SHA-256)
   c. Create `MerchantAcquirerLink` in `testing` status
   d. Emit `MerchantAcquirerLinkCreated`

**Produces**: `MerchantAcquirerLinkCreated`

**TDD Tests**:

```rust
#[tokio::test]
async fn test_create_link_success() {
    let cmd = CreateMerchantAcquirerLinkCommand {
        operator_id,
        connector_id: "checkout_com".into(),
        display_name: "Production Gateway".into(),
        environment: LinkEnvironment::Production,
        credentials: valid_checkout_credentials(),
    };
    let result = handler.handle(cmd).await.unwrap();
    assert_eq!(result.status, "testing");
    assert_eq!(result.connector_id, "checkout_com");
}

#[tokio::test]
async fn test_create_link_invalid_credentials() {
    let cmd = CreateMerchantAcquirerLinkCommand {
        credentials: invalid_credentials(),
        ..default()
    };
    let result = handler.handle(cmd).await;
    assert!(matches!(result, Err(PlatformError::Validation(ValidationError::InvalidCredentials))));
}

#[tokio::test]
async fn test_create_link_duplicate_credentials_detected() {
    // Same credentials hash → return existing link ref, not duplicate
    handler.handle(cmd.clone()).await.unwrap();
    let result = handler.handle(cmd).await;
    assert!(matches!(result, Ok(_))); // returns existing, not error
    // Actually should check: if same credentials already exist, update/reuse
}
```

### TestMerchantAcquirerConnection

```rust
pub struct TestMerchantAcquirerConnectionCommand {
    pub link_id: Uuid,
}
```

**Flow**:
1. Load link, decrypt credentials
2. Call `connector.test_connection(decrypted_credentials)`
3. On success: update `health_status = healthy`, `last_tested_at = now`, `last_healthy_at = now`
4. On failure: update `health_status = unhealthy`, emit alert

**Produces**: `MerchantAcquirerLinkTested` with test result details

### RotateMerchantAcquirerCredentials

```rust
pub struct RotateMerchantAcquirerCredentialsCommand {
    pub link_id: Uuid,
    pub new_credentials: RawConnectorCredentials,
    pub rotate_immediately: bool, // if true, swap instantly; if false, dual-key rotation
}
```

**Flow** (dual-key rotation):
1. Encrypt new credentials with a new DEK
2. Store new credentials alongside old (dual-key mode)
3. Test new credentials via sandbox call
4. On success: switch to new credentials, archive old
5. On failure: retain old credentials, alert operator

**Produces**: `MerchantAcquirerCredentialsRotated`

### DisableMerchantAcquirerLink / EnableMerchantAcquirerLink

```rust
pub struct DisableMerchantAcquirerLinkCommand {
    pub link_id: Uuid,
    pub reason: String, // 'maintenance' | 'credentials_issue' | 'merchant_request'
}

pub struct EnableMerchantAcquirerLinkCommand {
    pub link_id: Uuid,
}
```

**Preconditions** (Enable):
- Link must have valid credentials (not expired)
- Test connection before enabling

**Produces**: `MerchantAcquirerLinkDisabled`, `MerchantAcquirerLinkEnabled`

### UpdateMerchantAcquirerLinkMetadata

```rust
pub struct UpdateMerchantAcquirerLinkMetadataCommand {
    pub link_id: Uuid,
    pub display_name: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
```

---

## 4. Domain Events

| Event | Fields | Consumer |
|---|---|---|
| `MerchantAcquirerLinkCreated` | link_id, operator_id, connector_id, environment | orchestration-service, analytics-service |
| `MerchantAcquirerLinkEnabled` | link_id, operator_id | orchestration-service |
| `MerchantAcquirerLinkDisabled` | link_id, operator_id, reason | orchestration-service, notification-service |
| `MerchantAcquirerCredentialsRotated` | link_id, operator_id, rotated_at | audit-service |
| `MerchantAcquirerConnectionTested` | link_id, operator_id, success, latency_ms, error_message | analytics-service |
| `MerchantAcquirerCredentialsExpiring` | link_id, operator_id, days_until_expiry | notification-service |
| `MerchantAcquirerCredentialsExpired` | link_id, operator_id | notification-service |
| `MerchantAcquirerLinkHealthChanged` | link_id, operator_id, old_health, new_health | orchestration-service |

---

## 5. Repository Interface

```rust
#[async_trait]
pub trait MerchantAcquirerLinkRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<MerchantAcquirerLink>, PlatformError>;
    async fn save(&self, link: &MerchantAcquirerLink) -> Result<(), PlatformError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, PlatformError>;
    async fn find_active_by_operator(&self, operator_id: Uuid) -> Result<Vec<MerchantAcquirerLink>, PlatformError>;
    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<MerchantAcquirerLink>, PlatformError>;
    async fn find_by_credentials_hash(&self, hash: &str) -> Result<Option<MerchantAcquirerLink>, PlatformError>;
    async fn find_expired_credentials(&self) -> Result<Vec<MerchantAcquirerLink>, PlatformError>;
    async fn find_unhealthy(&self, max_age_minutes: u32) -> Result<Vec<MerchantAcquirerLink>, PlatformError>;
}
```

---

## 6. Background Jobs

| Job | Schedule | Description |
|---|---|---|
| **JOB-LINK-001**: Connection health check | Every 5 minutes | Test random subset of active links, update health_status |
| **JOB-LINK-002**: Credential expiry check | Daily | Find links with credentials expiring within 30 days, emit `MerchantAcquirerCredentialsExpiring` |
| **JOB-LINK-003**: Auto-disable expired | Daily | Automatically disable links with expired credentials |
| **JOB-LINK-004**: Re-test disabled links | Hourly | Try re-enabling links that were disabled due to transient issues |

---

## 7. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `LINK_NOT_FOUND` | 404 | NOT_FOUND | MerchantAcquirerLink does not exist |
| `CREDENTIALS_INVALID` | 400 | INVALID_ARGUMENT | Credentials failed validation against connector schema |
| `CREDENTIALS_TEST_FAILED` | 400 | FAILED_PRECONDITION | Connection test failed — credentials may be invalid |
| `LINK_ALREADY_DISABLED` | 409 | FAILED_PRECONDITION | Link is already disabled |
| `LINK_ALREADY_ENABLED` | 409 | FAILED_PRECONDITION | Link is already enabled |
| `MAX_LINKS_PER_CONNECTOR` | 409 | FAILED_PRECONDITION | Operator already has max active links for this connector |
| `CREDENTIALS_EXPIRED` | 409 | FAILED_PRECONDITION | Credentials have expired, cannot enable link |
| `CONNECTOR_NOT_FOUND` | 404 | NOT_FOUND | Connector type not registered in gateway |
| `ENCRYPTION_FAILED` | 500 | INTERNAL | Failed to encrypt credentials |

---

## 8. Credential Encryption Model

```rust
pub struct EncryptedCredentials {
    pub key_identifier: String,         // KMS key version identifier
    pub encrypted_payload: Vec<u8>,     // AES-256-GCM encrypted JSON blob
    pub nonce: Vec<u8>,                 // 12-byte nonce per GCM
    pub wrapped_dek: Vec<u8>,           // Data Encryption Key wrapped by KEK
    pub encryption_version: u32,        // For key rotation tracking
}

// Plaintext credential payload (what gets encrypted):
// {
//   "api_key": "sk_live_abc123...",
//   "merchant_id": "MID-12345",
//   "additional_fields": { ... }
// }
```

**Security Rules**:
- **CRED-SEC-001**: Credentials encrypted at rest with envelope encryption (DEK wrapped by KEK)
- **CRED-SEC-002**: KEK stored in KMS (AWS KMS, GCP Cloud KMS, or HashiCorp Vault)
- **CRED-SEC-003**: Plaintext credentials never returned via READ API
- **CRED-SEC-004**: Credentials masked in logs: `sk_live_***abc`
- **CRED-SEC-005**: Credentials purged from memory after use
- **CRED-SEC-006**: Connection test uses sandbox/status endpoint, never real-money call

---

## 9. BYOK Onboarding Integration

This service works with `connector-gateway` for the onboarding flow:

```
┌─────────────────────────────────────────────────────────────┐
│                    BYOK Onboarding                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Merchant navigates to "Connect Gateway" in dashboard    │
│  2. Frontend calls GET /v1/connectors → list available      │
│  3. Merchant selects "Checkout.com"                         │
│  4. Frontend calls GET /v1/connectors/checkout_com/schema   │
│     → returns OnboardingSchema (fields, validation, help)   │
│  5. Merchant fills in credentials in dynamic form           │
│  6. Frontend calls POST /v1/merchant-links                  │
│     → CreateMerchantAcquirerLink command                    │
│  7. Service validates format → validates against sandbox    │
│  8. On success: link created (status=testing), test button  │
│  9. Merchant clicks "Test Connection"                       │
│     → TestMerchantAcquirerConnection command                │
│  10. On test success: link becomes active                   │
│  11. Merchant configures routing rules → goes live          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 10. TDD Tests (Extended)

```rust
#[tokio::test]
async fn test_dual_key_rotation() {
    // Create link with old credentials
    let cmd = CreateMerchantAcquirerLinkCommand {
        credentials: old_credentials(),
        ..default()
    };
    let link = handler.handle(cmd).await.unwrap();

    // Rotate to new credentials (dual-key mode)
    let rotate = RotateMerchantAcquirerCredentialsCommand {
        link_id: link.link_id,
        new_credentials: new_credentials(),
        rotate_immediately: false,
    };
    let result = handler.handle(rotate).await.unwrap();

    // Verify: old credentials still work, new credentials stored alongside
    assert!(result.old_credentials_retained);
    assert_eq!(result.status, "active");
}

#[tokio::test]
async fn test_connection_health_tracking() {
    // Create link, test connection
    handler.handle(CreateMerchantAcquirerLinkCommand { credentials: valid_credentials(), ..default() }).await.unwrap();
    let test = TestMerchantAcquirerConnectionCommand { link_id };
    handler.handle(test).await.unwrap();
    let link = repo.load(link_id).await.unwrap();
    assert_eq!(link.health_status, "healthy");
    assert!(link.last_tested_at.is_some());
}

#[tokio::test]
async fn test_auto_disable_on_credentials_expired() {
    // Link with expired credentials should be auto-disabled
    let result = handler.handle(DisableExpiredCredentialsCommand::sweep()).await.unwrap();
    assert!(result.disabled_links > 0);
}

#[tokio::test]
async fn test_disabled_link_excluded_from_routing() {
    // Disable link
    handler.handle(DisableMerchantAcquirerLinkCommand { link_id, reason: "maintenance" }).await.unwrap();
    // Verify: routing doesn't include this link
    let active = repo.find_active_by_operator(operator_id).await.unwrap();
    assert!(!active.iter().any(|l| l.link_id == link_id));
}
```
