# 15 — analytics-service (BC-15 Analytics & Reporting)

Pure query-side read model. Consumes all domain events into ClickHouse.

---

## 1. Data Model (ClickHouse)

```sql
CREATE TABLE payment_events (
    event_type LowCardinality(String),
    payment_intent_id UUID,
    acquirer_id LowCardinality(String),
    card_scheme LowCardinality(String),
    currency LowCardinality(String),
    amount_minor_units Int64,
    decline_reason LowCardinality(String),
    latency_ms UInt32,
    occurred_at DateTime64(3),
    event_date Date MATERIALIZED toDate(occurred_at)
) ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (event_date, acquirer_id, card_scheme);
```

---

## 2. Materialized Views

```sql
CREATE MATERIALIZED VIEW auth_rate_hourly_mv
ENGINE = SummingMergeTree
PARTITION BY toYYYYMM(hour)
ORDER BY (hour, acquirer_id, card_scheme)
AS SELECT
    acquirer_id, card_scheme,
    toStartOfHour(occurred_at) AS hour,
    countIf(event_type = 'PaymentAuthorized') AS approved_count,
    countIf(event_type = 'PaymentFailed') AS declined_count
FROM payment_events
GROUP BY acquirer_id, card_scheme, hour;
```

---

## 3. Read Endpoints

- `GET /v1/analytics/authorization-rates` — hourly rates per acquirer/scheme
- `GET /v1/analytics/decline-reasons` — decline reason breakdown
- `GET /v1/analytics/settlement-status` — matched/unmatched counts
- `GET /v1/analytics/fee-analysis` — fees per acquirer (from ledger_entry)

---

## 4. Read Endpoints (Complete)

| Endpoint | Method | Description |
|---|---|---|
| `/v1/analytics/authorization-rates` | GET | Hourly rates per acquirer/scheme |
| `/v1/analytics/decline-reasons` | GET | Decline reason breakdown |
| `/v1/analytics/settlement-status` | GET | Matched/unmatched counts |
| `/v1/analytics/fee-analysis` | GET | Fees per acquirer (from ledger_entry) |
| `/v1/analytics/chargeback-trends` | GET | Chargeback rate by scheme (30/90 day) |
| `/v1/analytics/scheme-compliance` | GET | Visa/Mastercard threshold monitoring |
| `/v1/analytics/fraud-analysis` | GET | Fraud rate by BIN, geography, amount |
| `/v1/analytics/revenue-recovery` | GET | Revenue recovered via failover (GOAL-002) |

---

## 5. Error Catalog

| Code | HTTP | gRPC | Description |
|---|---|---|---|
| `ANALYTICS_UNAVAILABLE` | 503 | UNAVAILABLE | ClickHouse down |
| `STALE_DATA` | 503 | UNAVAILABLE | Data exceeds staleness threshold |
| `INVALID_DATE_RANGE` | 400 | INVALID_ARGUMENT | Date range validation failed |
| `QUERY_TIMEOUT` | 504 | DEADLINE_EXCEEDED | Query exceeded time limit |

---

## 6. TDD Tests

```rust
#[tokio::test]
async fn test_event_consumed_into_clickhouse() {
    // Publish PaymentAuthorized event
    // Query ClickHouse → verify row exists
}

#[tokio::test]
async fn test_auth_rate_materialized_view() {
    // Publish multiple events
    // Query auth_rate_hourly_mv → verify correct aggregation
}
```
