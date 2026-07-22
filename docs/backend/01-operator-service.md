# 01 — operator-service (BC-01 Operator Management)

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
CRUD + events. Owns Operator aggregate.

---

## 1. Domain Model

### AGG-Operator (Root)

**Identity**: `operator_id: Uuid`

**Entities**: `OperatorMember`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "operator")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,           // CHAR(2), default 'AE'
    pub status: String,            // 'pending' | 'active_unverified' | 'active_verified' | 'suspended' | 'expired_unverified'
    pub subdomain: String,
    pub provisioned_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}
```

---

## 2. Commands

### RegisterOperator

```rust
pub struct RegisterOperatorCommand {
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub email: String,
}
```

**Preconditions**:
- Trade license number passes format validation (AF-001a)
- Subdomain not already taken (AF-001b)

**Flow**:
1. Create Operator in `Pending` status
2. Send email verification
3. On verification → transition to `Active-Unverified`
4. Provision default ABAC roles

**Produces**: `OperatorRegistered`

### VerifyEmail

**Produces**: `OperatorVerified` (status → `Active-Unverified`)

### UpdateOperatorStatus

```rust
pub struct UpdateOperatorStatusCommand {
    pub operator_id: Uuid,
    pub new_status: String,
    pub reason: String,
}
```

**Preconditions**:
- Only callable by compliance-service (internal, via gRPC)
- Status transitions follow valid paths

**Produces**: `OperatorSuspended` | `OperatorVerified`

---

## 3. Business Rules

- **BR-001-1**: Cannot process live transactions until KYB status = `Approved`
- **BR-001-2**: Sandbox access available immediately post email-verification
- **PROV-002**: Provisioning is idempotent

---

## 4. Repository Interface

```rust
#[async_trait]
pub trait OperatorRepository: Send + Sync {
    async fn load(&self, id: OperatorId) -> Result<Option<Operator>, PlatformError>;
    async fn save(&self, operator: &Operator) -> Result<(), PlatformError>;
    async fn find_by_trade_license(&self, license: &str) -> Result<Option<Operator>, PlatformError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Operator>, PlatformError>;
}
```

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `OPERATOR_NOT_FOUND` | 404 | NOT_FOUND | Operator does not exist |
| `DUPLICATE_TRADE_LICENSE` | 409 | ALREADY_EXISTS | Trade license already registered |
| `DUPLICATE_SUBDOMAIN` | 409 | ALREADY_EXISTS | Subdomain already taken |
| `INVALID_TRADE_LICENSE_FORMAT` | 400 | INVALID_ARGUMENT | Trade license fails format validation |
| `EMAIL_NOT_VERIFIED` | 403 | FAILED_PRECONDITION | Email verification pending |
| `KYB_NOT_APPROVED` | 403 | FAILED_PRECONDITION | KYB not yet approved |
| `OPERATOR_SUSPENDED` | 403 | FAILED_PRECONDITION | Operator account suspended |

---

## 6. TDD Tests

```rust
#[tokio::test]
async fn test_register_operator_success() {
    let result = handler.handle(RegisterOperatorCommand {
        legal_name: "Acme Corp".into(),
        trade_license_no: "CN-12345".into(),
        country: "AE".into(),
        email: "admin@acme.com".into(),
    }).await.unwrap();
    assert_eq!(result.status, "pending");
}

#[tokio::test]
async fn test_register_duplicate_trade_license_rejected() {
    handler.handle(RegisterOperatorCommand { trade_license_no: "CN-12345".into(), ... }).await.unwrap();
    let result = handler.handle(RegisterOperatorCommand { trade_license_no: "CN-12345".into(), ... }).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_email_verification_transitions_to_active_unverified() {
    let op = handler.handle(RegisterOperatorCommand { ... }).await.unwrap();
    handler.handle(VerifyEmailCommand { operator_id: op.id }).await.unwrap();
    let updated = repository.load(op.id).await.unwrap();
    assert_eq!(updated.status, "active_unverified");
}

#[tokio::test]
async fn test_provisioning_idempotent() {
    // Run provisioning twice for same operator → no error, no duplicate resources
}
```
