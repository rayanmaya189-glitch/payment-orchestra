# 03 — compliance-service (BC-03 Merchant Compliance)

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
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

## 3. Repository Interface

```rust
#[async_trait]
pub trait KybCaseRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<KybCase>, PlatformError>;
    async fn save(&self, case: &KybCase) -> Result<(), PlatformError>;
    async fn find_pending_review(&self) -> Result<Vec<KybCase>, PlatformError>;
}

#[async_trait]
pub trait AmlAlertRepository: Send + Sync {
    async fn save(&self, alert: &AmlAlert) -> Result<(), PlatformError>;
    async fn find_open(&self, operator_id: Uuid) -> Result<Vec<AmlAlert>, PlatformError>;
    async fn find_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<AmlAlert>, PlatformError>;
}
```

---

## 4. AML Transaction Monitoring (Part 8 §11.1)

### AML Alert Entity

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "aml_alert")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub alert_id: Uuid,
    pub operator_id: Uuid,
    pub transaction_id: Uuid,
    pub alert_type: String,     // 'structuring' | 'velocity' | 'amount_anomaly' | 'rapid_succession'
    pub severity: String,       // 'low' | 'medium' | 'high' | 'critical'
    pub rule_id: String,        // which AML rule triggered
    pub details: String,        // JSON with rule-specific details
    pub status: String,         // 'open' | 'under_review' | 'escalated' | 'closed'
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}
```

### AML Rules (AML-001)

```rust
pub struct AmlRule {
    pub rule_id: String,
    pub rule_type: AmlRuleType,
    pub threshold: AmlThreshold,
    pub window: Duration,
    pub severity: String,
}

pub enum AmlRuleType {
    /// Multiple transactions just below reporting threshold within window
    Structuring {
        report_threshold_minor_units: i64,
        near_threshold_percent: f64, // e.g., 0.9 (90% of threshold)
        min_transactions: u32,
        window_minutes: u32,
    },
    /// Unusual transaction frequency or volume
    Velocity {
        max_count: u32,
        max_amount_minor_units: i64,
        window_minutes: u32,
    },
    /// Transactions significantly exceeding operator's average
    AmountAnomaly {
        multiplier: f64, // e.g., 10x average
        min_sample_size: u32,
    },
    /// Multiple authorizations on same payment method in short window
    RapidSuccession {
        max_count: u32,
        window_seconds: u32,
    },
}
```

### SAR Generation (AML-003)

```rust
pub struct SarReport {
    pub report_id: Uuid,
    pub operator_id: Uuid,
    pub alert_ids: Vec<Uuid>,
    pub transactions: Vec<SarTransaction>,
    pub narrative: String,
    pub generated_at: DateTime<Utc>,
    pub status: String, // 'draft' | 'submitted' | 'filed'
}

pub struct SarTransaction {
    pub transaction_id: Uuid,
    pub amount: Money,
    pub currency: CurrencyCode,
    pub timestamp: DateTime<Utc>,
    pub counterparty: Option<String>,
    pub description: String,
}
```

### Concrete AML Rules (Production Defaults)

| Rule ID | Type | Threshold | Window | Severity |
|---|---|---|---|---|
| AML-R001 | Structuring | 5+ txns between 45,000–49,999 AED (90% of 50K threshold) | 60 min | high |
| AML-R002 | Structuring | 3+ txns between 90,000–99,999 AED (90% of 100K threshold) | 60 min | critical |
| AML-R003 | Velocity | 20+ transactions from same operator | 60 min | medium |
| AML-R004 | Velocity | 50+ transactions from same operator | 60 min | high |
| AML-R005 | Amount Anomaly | Single tx > 10x operator's 30-day average | Per tx | high |
| AML-R006 | Amount Anomaly | Single tx > 500,000 AED regardless of average | Per tx | critical |
| AML-R007 | Rapid Succession | 5+ authorizations on same card BIN within 5 min | 5 min | medium |
| AML-R008 | Rapid Succession | 10+ authorizations on same card BIN within 5 min | 5 min | high |
| AML-R009 | Cross-Border | 3+ transactions to high-risk jurisdictions (FATF grey list) | 24 hr | high |
| AML-R010 | Structuring | 10+ zero-amount authorizations (card testing indicator) | 60 min | critical |

**Configurable per operator**: Thresholds, windows, and severity levels are operator-configurable via the compliance dashboard. Defaults above are platform-wide minimums.

### Compliance Monitor (Background Job)

```rust
pub struct AmlMonitor {
    rule_set: Vec<AmlRule>,
    alert_repository: Box<dyn AmlAlertRepository>,
    notification_service: NotificationClient,
}

impl AmlMonitor {
    pub async fn scan_transaction(&self, event: &PaymentAuthorized) -> Result<Vec<AmlAlert>, PlatformError> {
        let mut alerts = vec![];

        for rule in &self.rule_set {
            match &rule.rule_type {
                AmlRuleType::Structuring { report_threshold_minor_units, near_threshold_percent, min_transactions, window_minutes } => {
                    let recent = self.get_recent_transactions(event.operator_id, *window_minutes).await?;
                    let near_threshold = recent.iter()
                        .filter(|t| t.amount_minor_units >= (report_threshold_minor_units as f64 * near_threshold_percent) as i64
                            && t.amount_minor_units < *report_threshold_minor_units)
                        .count();
                    if near_threshold >= *min_transactions as usize {
                        alerts.push(self.create_alert(event, rule, "structuring"));
                    }
                }
                AmlRuleType::Velocity { max_count, max_amount_minor_units, window_minutes } => {
                    let recent = self.get_recent_transactions(event.operator_id, *window_minutes).await?;
                    if recent.len() >= *max_count as usize {
                        alerts.push(self.create_alert(event, rule, "velocity"));
                    }
                }
                AmlRuleType::AmountAnomaly { multiplier, min_sample_size } => {
                    let avg = self.get_average_transaction_amount(event.operator_id, *min_sample_size).await?;
                    if event.amount.amount_minor_units > (avg as f64 * multiplier) as i64 {
                        alerts.push(self.create_alert(event, rule, "amount_anomaly"));
                    }
                }
                AmlRuleType::RapidSuccession { max_count, window_seconds } => {
                    let recent = self.get_recent_by_payment_method(event.payment_method_id, *window_seconds).await?;
                    if recent.len() >= *max_count as usize {
                        alerts.push(self.create_alert(event, rule, "rapid_succession"));
                    }
                }
            }
        }

        for alert in &alerts {
            self.alert_repository.save(alert).await?;
            self.notification_service.send_aml_alert(alert).await?;
        }

        Ok(alerts)
    }
}
```

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `KYB_CASE_NOT_FOUND` | 404 | NOT_FOUND | KYB case does not exist |
| `KYB_CASE_ALREADY_RESOLVED` | 409 | FAILED_PRECONDITION | Case already approved/rejected |
| `KYB_NO_DOCUMENTS` | 400 | INVALID_ARGUMENT | At least one document required |
| `AML_ALERT_NOT_FOUND` | 404 | NOT_FOUND | AML alert does not exist |
| `AML_ALERT_ALREADY_REVIEWED` | 409 | FAILED_PRECONDITION | Alert already reviewed |
| `SAR_GENERATION_FAILED` | 500 | INTERNAL | SAR report generation failed |
| `PARTNER_API_UNAVAILABLE` | 503 | UNAVAILABLE | KYB partner API down |

---

## 6. TDD Tests

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
