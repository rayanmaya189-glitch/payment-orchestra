# 01 — operator-service (BC-01 Operator Management)

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

## 4. TDD Tests

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
