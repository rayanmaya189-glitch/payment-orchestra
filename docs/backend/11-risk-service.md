# 11 — risk-service (BC-11 Fraud & Risk Scoring)

MVP: Rule-based scoring. H3: ML-based scoring.

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

## 2. Risk Rules (MVP)

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
}
```

**Returns**: `RiskAssessment` with score and factors

---

## 4. Repository Interface

```rust
#[async_trait]
pub trait RiskAssessmentRepository: Send + Sync {
    async fn load_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<RiskAssessment>, PlatformError>;
    async fn save(&self, assessment: &RiskAssessment) -> Result<(), PlatformError>;
    async fn find_high_risk(&self, operator_id: Uuid, since: DateTime<Utc>) -> Result<Vec<RiskAssessment>, PlatformError>;
}
```

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
