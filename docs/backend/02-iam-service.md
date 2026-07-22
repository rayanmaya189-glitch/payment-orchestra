# 02 — iam-service (BC-02 Identity & Access)

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
Owns authentication, authorization (ABAC), session management, and API key lifecycle.

---

## 1. Domain Model

### Aggregates

#### AGG-Principal (Root)

**Identity**: `principal_id: Uuid`

**Entities**: `RoleAssignment`, `MfaEnrollment`, `BackupCode`

**Value Objects**: `Permission`, `Role`, `AccessCondition`, `SessionToken`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "principal")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub principal_type: String,    // 'human' | 'api_key' | 'service'
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>,  // argon2id
    pub mfa_enrolled: bool,
    pub mfa_method: Option<String>,      // 'webauthn' | 'totp'
    pub status: String,            // 'active' | 'suspended' | 'deleted'
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub last_login_at: Option<DateTimeWithTimeZone>,
}
```

#### AGG-PendingChange (Maker/Checker)

**Identity**: `change_id: Uuid`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pending_changes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub change_id: Uuid,
    pub change_type: String,
    pub maker_id: Uuid,
    pub checker_id: Option<Uuid>,
    pub payload: Vec<u8>,
    pub status: String,            // 'pending' | 'approved' | 'rejected' | 'expired'
    pub maker_note: Option<String>,
    pub checker_note: Option<String>,
    pub requested_at: DateTimeWithTimeZone,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    pub expires_at: DateTimeWithTimeZone,
    pub created_at: DateTimeWithTimeZone,
}
```

---

## 2. Commands

### Authenticate (Email/Password)

```rust
pub struct AuthenticateCommand {
    pub email: String,
    pub password: String,
    pub ip_address: IpAddr,
    pub user_agent: String,
}
```

**Preconditions**:
- Account not locked (AUTH-007)
- Rate limit: 10 attempts/IP/minute (AUTH-009)
- Password verified against Argon2id hash
- HIBP breach check on password creation/change (AUTH-003)

**Produces**: `PrincipalAuthenticated` event, JWT access token + refresh token

**TDD Tests**:

```rust
#[tokio::test]
async fn test_authenticate_success() {
    let result = handler.handle(AuthenticateCommand {
        email: "admin@example.com".into(),
        password: "SecureP@ss123".into(),
        ip_address: "1.2.3.4".parse().unwrap(),
        user_agent: "Mozilla/5.0".into(),
    }).await.unwrap();
    assert!(!result.access_token.is_empty());
    assert!(!result.refresh_token.is_empty());
}

#[tokio::test]
async fn test_authenticate_wrong_password() {
    let result = handler.handle(AuthenticateCommand {
        email: "admin@example.com".into(),
        password: "wrong".into(),
        ip_address: "1.2.3.4".parse().unwrap(),
        user_agent: "Mozilla/5.0".into(),
    }).await;
    assert!(matches!(result, Err(PlatformError::AuthorizationDenied(_))));
}

#[tokio::test]
async fn test_authenticate_lockout_after_5_failures() {
    for _ in 0..5 {
        handler.handle(AuthenticateCommand {
            email: "admin@example.com".into(),
            password: "wrong".into(),
            ip_address: "1.2.3.4".parse().unwrap(),
            user_agent: "Mozilla/5.0".into(),
        }).await.ok();
    }
    let result = handler.handle(AuthenticateCommand {
        email: "admin@example.com".into(),
        password: "SecureP@ss123".into(), // correct password
        ip_address: "1.2.3.4".parse().unwrap(),
        user_agent: "Mozilla/5.0".into(),
    }).await;
    assert!(matches!(result, Err(PlatformError::AuthorizationDenied(_))));
}

#[tokio::test]
async fn test_authenticate_lockout_expires_after_15_minutes() {
    // Simulate 5 failures, advance time 16 minutes, try again
    // Should succeed
}
```

### IssueToken (Refresh)

```rust
pub struct IssueTokenCommand {
    pub refresh_token: String,
}
```

**Preconditions**:
- Refresh token exists in server-side store
- Not expired
- Single-use: old token invalidated on use (AUTH-002)

**Produces**: New access token + new refresh token (rotation)

### ValidatePermission

```rust
pub struct ValidatePermissionQuery {
    pub principal_id: Uuid,
    pub resource: String,
    pub action: String,
    pub context: PermissionContext, // amount, acquirer_link_id, etc.
}
```

**TDD Tests**:

```rust
#[tokio::test]
async fn test_validate_permission_allowed() {
    let result = handler.handle(ValidatePermissionQuery {
        principal_id: admin_id,
        resource: "payment_intents".into(),
        action: "capture".into(),
        context: PermissionContext { amount: Some(10000) },
    }).await.unwrap();
    assert!(result.allowed);
}

#[tokio::test]
async fn test_validate_permission_denied_above_threshold() {
    // Finance Operator trying to refund above 50,000 AED threshold
    let result = handler.handle(ValidatePermissionQuery {
        principal_id: finance_id,
        resource: "payment_intents".into(),
        action: "refund".into(),
        context: PermissionContext { amount: Some(60000) },
    }).await.unwrap();
    assert!(!result.allowed);
    assert!(result.requires_maker_checker);
}
```

### CreateApiKey / RotateApiKey / RevokeApiKey

```rust
pub struct CreateApiKeyCommand {
    pub principal_id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub acquirer_link_ids: Option<Vec<Uuid>>, // AUTH-010 scoping
    pub expires_in_days: Option<u32>, // default: 90
}
```

**Preconditions**:
- Maker/Checker approval required (APIKEY-MKCK-001)
- API key hashed with Argon2id (AUTH-005)

**TDD Tests**:

```rust
#[tokio::test]
async fn test_create_api_key_success() {
    let result = handler.handle(CreateApiKeyCommand {
        principal_id: dev_id,
        name: "Production API Key".into(),
        scopes: vec!["payments:read".into(), "payments:write".into()],
        acquirer_link_ids: None,
        expires_in_days: Some(90),
    }).await.unwrap();
    assert!(!result.api_key_secret.is_empty()); // only returned on creation
    assert!(!result.api_key_id.is_empty());
}

#[tokio::test]
async fn test_revoke_api_key() {
    handler.handle(CreateApiKeyCommand { ... }).await.unwrap();
    let result = handler.handle(RevokeApiKeyCommand { api_key_id }).await.unwrap();
    assert!(result.revoked);
}
```

---

## 3. Domain Events

| Event | Fields | Consumer |
|---|---|---|
| `PrincipalCreated` | principal_id, email, principal_type | Audit |
| `PrincipalAuthenticated` | principal_id, ip_address, user_agent | Security monitoring |
| `RoleAssigned` | principal_id, role | Permission cache invalidation |
| `PermissionDenied` | principal_id, resource, action, context | Security alerting |
| `ApiKeyCreated` | api_key_id, principal_id, scopes | Audit |
| `ApiKeyRevoked` | api_key_id, principal_id | Permission cache invalidation |

---

## 4. Security Controls

- **AUTH-007**: 5 failed → 15min lockout; 10 → 1hr; 20 → suspension
- **AUTH-008**: Session timeout: 30min (Admin/Finance), 60min (Developer/ReadOnly)
- **AUTH-011**: WebAuthn minimum for Admin/Finance (not just TOTP)
- **AUTH-012**: WebAuthn attestation verification on enrollment
- **AUTH-013**: Session token includes hash of credential_id
- **AUTH-014**: 10 backup codes generated at MFA enrollment (argon2id hashed)
- **AUTH-015**: Login notification on new device/IP
- **AUTH-016**: All sessions invalidated on password change
- **AUTH-017**: JWT: RS256 or ES256 only; reject `none`/`HS256`
- **AUTH-018**: Backup code: 5 failed/15min → lockout
- **AUTH-019**: Reject credentials in URL query strings
- **SESS-SEC-001**: Cookies: SameSite=Strict; Secure; HttpOnly
- **SESS-SEC-002**: JWT `aud` claim binding to client type
- **SESS-SEC-003**: Refresh token rotation with concurrent session detection
- **SESS-SEC-004**: Admin session revocation endpoint
- **CSRF-001**: CSRF token on state-changing endpoints with cookie auth
- **ABAC-002**: Step-up re-auth for sensitive actions (acquirer credential changes)
- **ABAC-009**: IP allowlisting for sensitive operations (KEK ceremonies, DB access)
- **AUTHZ-001**: PermissionDenied events logged + alerting on repeated denials
- **PAM-004**: Break-glass support access (support-reader role, 2hr auto-expiry)

### Missing Commands (Added per Gap Analysis)

```rust
pub struct InvalidateAllSessionsCommand {
    pub principal_id: Uuid, // triggered on password change (AUTH-016)
}

pub struct SendLoginNotificationCommand {
    pub principal_id: Uuid,
    pub ip_address: IpAddr,
    pub user_agent: String,
    pub is_new_device: bool, // AUTH-015
}

pub struct RevokeAllSessionsCommand {
    pub operator_id: Uuid, // Admin session revocation (SESS-SEC-004)
}
```

### Missing TDD Tests

```rust
#[tokio::test]
async fn test_maker_checker_self_approval_rejected() {
    let result = handler.handle(ApprovePendingChangeCommand {
        change_id,
        checker_id: maker_id, // same as maker
    }).await;
    assert!(matches!(result, Err(PlatformError::AuthorizationDenied(_))));
}

#[tokio::test]
async fn test_pending_change_auto_expires() {
    // Create pending change, advance time past timeout
    // Verify status transitions to 'expired'
}

#[tokio::test]
async fn test_permission_denied_event_published() {
    // Attempt unauthorized operation
    // Verify PermissionDenied event emitted
}

#[tokio::test]
async fn test_all_sessions_invalidated_on_password_change() {
    // Create 3 sessions for same principal
    // Change password
    // Verify all 3 sessions are invalid
}

#[tokio::test]
async fn test_webauthn_mfa_enforced_for_admin() {
    // Admin without WebAuthn → cannot authenticate
    // Admin with TOTP only → step-up required
}
```
