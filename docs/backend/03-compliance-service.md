# 03 — compliance-service (BC-03 Merchant Compliance)

KYB workflow. ACL against external KYB partner API.

---

## 1. Domain Model

### AGG-KybCase (Root)

**Identity**: `kyb_case_id: Uuid`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "kyb_case")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub kyb_case_id: Uuid,
    pub operator_id: Uuid,
    pub status: String, // 'submitted' | 'under_review' | 'approved' | 'rejected'
    pub submitted_by: Uuid,
    pub documents: String,          // JSON array of document_ids
    pub ocr_extracted_fields: Option<String>,
    pub partner_decision: Option<String>,
    pub rejection_reason: Option<String>,
    pub submitted_at: DateTimeWithTimeZone,
    pub resolved_at: Option<DateTimeWithTimeZone>,
}
```

---

## 2. Commands

### SubmitKybEvidence

```rust
pub struct SubmitKybEvidenceCommand {
    pub operator_id: Uuid,
    pub document_ids: Vec<Uuid>,
}
```

**Preconditions**:
- Operator in `Active-Unverified` state
- At least one document uploaded

**Flow**:
1. Create KybCase in `Submitted` status
2. Trigger OCR on each document (via document-service)
3. Route to partner API or internal review queue
4. On approval → transition operator to `Active-Verified`

**Produces**: `KybCaseSubmitted`

### ReviewKybCase

```rust
pub struct ReviewKybCaseCommand {
    pub kyb_case_id: Uuid,
    pub decision: KybDecision, // Approved | Rejected
    pub reason: Option<String>,
}
```

**Produces**: `KybCaseApproved` or `KybCaseRejected`

---

## 3. TDD Tests

```rust
#[tokio::test]
async fn test_submit_kyb_evidence() {
    let result = handler.handle(SubmitKybEvidenceCommand {
        operator_id,
        document_ids: vec![doc_id],
    }).await.unwrap();
    assert_eq!(result.status, "submitted");
}

#[tokio::test]
async fn test_kyb_approval_transitions_operator() {
    // Submit → Approve → verify operator status is Active-Verified
}

#[tokio::test]
async fn test_kyb_rejection_allows_resubmission() {
    // Submit → Reject → Submit new evidence → new KybCase created
}
```
