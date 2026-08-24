# 11 — risk-service (BC-11 Fraud & Risk Scoring)

> ⚡ **Pure Router**: The platform is a routing and orchestration layer only. Funds flow directly between the customer, the payment gateway, and the merchant bank account. The platform never holds, touches, or controls funds.

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
Initial release: Rule-based scoring. Future: ML-based scoring.

---

## 1. Domain Model

### AGG-RiskAssessment (Root)

**Identity**: `risk_assessment_id: Uuid`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "risk_assessment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub risk_assessment_id: Uuid,
    pub payment_intent_id: Uuid,
    pub risk_score: f64,           // 0.0 - 1.0
    pub risk_level: String,        // 'low' | 'medium' | 'high' | 'critical'
    pub risk_factors: String,      // JSON array of risk factor descriptions
    pub rule_version: String,
    pub assessed_at: DateTimeWithTimeZone,
}
```

---

## 2. Risk Rules (Phase 1)

```rust
pub struct RiskRule {
    pub rule_id: String,
    pub condition: RiskCondition,
    pub score_increment: f64,
    pub description: String,
}

pub enum RiskCondition {
    HighAmount { threshold: i64 },
    HighVelocity { max_count: u32, window_minutes: u32 },
    SuspiciousBin { bin_prefixes: Vec<String> },
    GeoMismatch { billing_shipping_mismatch: bool },
    NewPaymentMethod { max_age_days: u32 },
}
```

---

## 3. Commands

### AssessRisk

```rust
pub struct AssessRiskQuery {
    pub payment_intent_id: Uuid,
    pub amount: Money,
    pub card_bin: String,
    pub ip_address: IpAddr,
    pub billing_country: String,
    pub shipping_country: Option<String>,
    pub payment_method_token_id: Option<Uuid>, // Gap: token age/reuse factor
    pub source_type: SourceType,               // Gap: channel risk factor
}
```

**Returns**: `RiskAssessment` with score and factors

### Gap: Integration with Orchestration Service

The `risk-service` is called synchronously by `orchestration-service` during `AuthorizePaymentIntent` (before routing selection):

```
1. orchestration-service receives AuthorizePaymentIntent
2. Before routing algorithm runs, call risk-service via gRPC:
   - risk-service AssessRisk(payment_intent_id, amount, card_bin, ip, billing_country, ...)
3. Risk score returned:
   - score < 0.3 (low): proceed with normal routing
   - score 0.3-0.7 (medium): proceed, but log risk factor for analytics
   - score 0.7-0.9 (high): route to acquirer with best fraud screening (per GatewayProfile)
   - score > 0.9 (critical): reject with HIGH_RISK_DECLINED (configurable per operator)
4. Risk score stored on PaymentIntent for audit trail
5. RiskScoreAssigned event emitted for analytics
```

### Gap: Risk-Based Routing Rule

`RoutingPolicy` can include risk-based routing rules:

```rust
pub struct RiskBasedRoutingRule {
    pub min_risk_score: f64,          // minimum risk score to trigger this rule
    pub preferred_acquirer_link_id: Uuid, // route high-risk to this acquirer
    pub fallback_reject: bool,        // if true, reject if preferred acquirer unavailable
}
```

**How it works**:
1. Routing algorithm evaluates normal rules first (card scheme, currency, amount)
2. If risk_score > rule.min_risk_score, override normal priority with preferred acquirer
3. If preferred acquirer is down (circuit breaker open) and fallback_reject = true → reject
4. If preferred acquirer is down and fallback_reject = false → proceed with normal routing

---

## 4. Repository Interface

```rust
#[async_trait]
pub trait RiskAssessmentRepository: Send + Sync {
    async fn load_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<RiskAssessment>, PlatformError>;
    async fn save(&self, assessment: &RiskAssessment) -> Result<(), PlatformError>;
    async fn find_high_risk(&self, operator_id: Uuid, since: DateTime<Utc>) -> Result<Vec<RiskAssessment>, PlatformError>;
    async fn get_risk_stats(&self, operator_id: Uuid, window_hours: u32) -> Result<RiskStats, PlatformError>;
}

// Gap: risk statistics for routing optimization
pub struct RiskStats {
    pub avg_risk_score: f64,
    pub high_risk_count: u32,
    pub critical_risk_count: u32,
    pub risk_by_bin: HashMap<String, f64>,    // avg risk score by BIN prefix
    pub risk_by_country: HashMap<String, f64>, // avg risk score by country
}
```

---

## 5. Domain Events

| Event | Consumer |
|---|---|
| `RiskScoreAssigned` | analytics-service, ai-assistant-service |
| `TransactionFlaggedHighRisk` | notification-service, analytics-service |

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `RISK_ASSESSMENT_NOT_FOUND` | 404 | NOT_FOUND | Risk assessment does not exist |
| `INVALID_CARD_BIN` | 400 | INVALID_ARGUMENT | Card BIN must be 6 digits |
| `UNSUPPORTED_CURRENCY` | 400 | INVALID_ARGUMENT | Currency not in supported list |

---

## 6. TDD Tests

```rust
#[tokio::test]
async fn test_low_risk_transaction() {
    let result = handler.handle(AssessRiskQuery {
        amount: Money { amount_minor_units: 5000, currency: AED },
        card_bin: "411111".into(),
        ip_address: "1.2.3.4".parse().unwrap(),
        billing_country: "AE".into(),
        shipping_country: Some("AE".into()),
    }).await.unwrap();
    assert!(result.risk_score < 0.3);
    assert_eq!(result.risk_level, "low");
}

#[tokio::test]
async fn test_high_risk_transaction() {
    let result = handler.handle(AssessRiskQuery {
        amount: Money { amount_minor_units: 500000, currency: AED },
        card_bin: "411111".into(),
        ip_address: "1.2.3.4".parse().unwrap(),
        billing_country: "AE".into(),
        shipping_country: Some("US".into()), // geo mismatch
    }).await.unwrap();
    assert!(result.risk_score > 0.5);
}
```


