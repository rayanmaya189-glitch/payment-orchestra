# Software Requirements Specification
## AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

**Document Series:** 12-Part Enterprise SRS
**Part 9 of 12:** Database Design (PostgreSQL, Redis, ClickHouse, OpenSearch, MinIO)
**Document Status:** Draft v0.1
**Classification:** Confidential — Internal Engineering & Product Use

---

## 0. Document Control

| Field | Value |
|---|---|
| Part | 9 of 12 — Database Design |
| Depends On | Part 3 (aggregates/events), Part 4 (per-service datastore ownership), Part 5 (event store needs), Part 6 (vector index needs), Part 7 (settlement staging), Part 8 (encryption/audit storage requirements) |
| Feeds Into | Part 10 (API contracts reflect these schemas), Part 11 (backup/DR, capacity planning, migration/CI practices) |
| Golden Rule | Every table/index is designed for single-tenant deployment — no tenant_id columns needed. Access control is enforced at the application layer via ABAC (Part 8), not at the data layer. |

---

## 1. PostgreSQL — Event Store Design (Event-Sourced Contexts)

### 1.1 Shared Event Store Schema (SeaORM — Rust)

Each event-sourced service (BC-05 `orchestration-service`, BC-08 `subscription-service`, BC-09 `reconciliation-service`, BC-10 `dispute-service`) owns its own Postgres database using the same SeaORM entity:

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "event_store")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub aggregate_type: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub aggregate_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_sequence: i64,
    pub event_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub occurred_at: DateTimeWithTimeZone,
    pub actor_type: String,       // 'user' | 'api_key' | 'system'
    pub actor_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub payload: Vec<u8>,          // protobuf-encoded (Part 10)
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

**Indexes (defined via SeaORM migration):**

```rust
// In SeaORM migration file
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("event_store_event_id_uq")
                    .table(EventStore::Table)
                    .col(EventStore::EventId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("event_store_correlation_idx")
                    .table(EventStore::Table)
                    .col(EventStore::CorrelationId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("event_store_occurred_at_idx")
                    .table(EventStore::Table)
                    .col(EventStore::OccurredAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
```

- **DB-001 (Optimistic concurrency)**: Appends specify `expected_event_sequence`; the insert is conditioned (application-level check-then-insert within a transaction, or a Postgres `EXCLUDE`/unique-constraint-based guard on `(aggregate_type, aggregate_id, event_sequence)`) so a concurrent writer's stale-sequence append fails and must reload+retry (Part 5 §4.2 CONC-001).
- **DB-002 (Payload encoding)**: `payload` is protobuf, not JSON, for compactness and schema evolution discipline (Part 10 defines `.proto` schemas per event type, with explicit backward-compatibility rules — additive fields only within a version, breaking changes bump `event_version`).
- **DB-003 (Snapshotting)**: For aggregates with long event streams (e.g., a `Subscription` that has renewed monthly for years), a `aggregate_snapshot` entity stores periodic materialized state to bound replay cost:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "aggregate_snapshot")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub aggregate_type: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub aggregate_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub as_of_sequence: i64,
    pub state: Vec<u8>,           // protobuf-encoded materialized aggregate state
    pub created_at: DateTimeWithTimeZone,
}
```

### 1.2 Read-Model (Projection) Entities (SeaORM — Rust)

Projections are plain, indexed-for-query Postgres tables (or ClickHouse for heavy analytics, §3), rebuilt from the event stream and kept eventually consistent (Part 3 §7 CQRS). Representative example — the reconciliation exception queue (UC-041):

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "reconciliation_exception_projection")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub exception_id: Uuid,
    pub settlement_record_id: Uuid,
    pub payment_intent_id: Option<Uuid>,
    pub amount_minor_units: i64,
    pub currency: String,        // CHAR(3)
    pub status: String,          // 'unmatched' | 'resolved' | 'flagged_discrepancy'
    pub detected_at: DateTimeWithTimeZone,
    pub resolved_at: Option<DateTimeWithTimeZone>,
    pub resolved_by_actor_id: Option<Uuid>,
}
```

### 1.3 Non-Event-Sourced Service Entities (SeaORM — Rust)

For BC-01/02/03/13/14 (Tier-2-audited per Part 8 §5.1), tables use SeaORM entities:

```rust
// SeaORM entity for operator (Rust)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "operator")]
pub struct OperatorModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,           // CHAR(2), default 'AE'
    pub status: String,            // pending | active_unverified | active_verified | suspended | expired_unverified
    pub created_at: DateTimeWithTimeZone,
}

// SeaORM entity for principal (Rust)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "principal")]
pub struct PrincipalModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub principal_type: String,    // human | api_key | service
    pub email: Option<String>,
    pub password_hash: Option<Vec<u8>>,  // argon2id
    pub mfa_enrolled: bool,
    pub status: String,            // active | suspended | deleted
    pub created_at: DateTimeWithTimeZone,
}

// SeaORM entity for role_assignment (Rust)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "role_assignment")]
pub struct RoleAssignmentModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub principal_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub role: String,
    pub abac_conditions: String,   // JSON: {"max_refund_amount_minor": 500000}
}

// SeaORM entity for audit_log (Rust, append-only)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_log")]
pub struct AuditLogModel {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub before_state: Option<String>,  // JSON
    pub after_state: Option<String>,   // JSON
    pub occurred_at: DateTimeWithTimeZone,
    pub source_ip: Option<String>,
}
```

- **DB-004**: `audit_log` is append-only at the database-privilege level — the application's database role for these services is granted `INSERT`/`SELECT` only on `audit_log`, with no `UPDATE`/`DELETE` grant at all (Part 8 §5.3 AUD-003 enforced at the DB-permission layer). SeaORM generates no `Update`/`Delete` methods for this entity by design (custom behavior override).

- **DB-011 (Row-Level Security)**: All Postgres tables containing operator data implement Row-Level Security (RLS) policies as a defense-in-depth layer. Even if the application layer has an authorization bypass, the database enforces that queries can only access data belonging to the authenticated principal's operator context. RLS policies are applied at the table level and enforced by Postgres row-security policies, not application logic.

- **DB-012 (Database Activity Monitoring)**: All database queries are logged to a tamper-evident audit trail (separate from the application audit log). Queries accessing Restricted-classification data (Part 8 ENC-009) trigger alerts. Database connection attempts from unauthorized source IPs are rejected and logged.

- **DB-013 (Database Connection Security)**: Database connections use TLS 1.3 (matching Part 8 ENC-001). Database credentials are rotated automatically via the secrets management system (Part 8 SEC-003). No application hardcodes database credentials.

### 1.4 Encrypted Field Storage (SeaORM — Rust, Ties to Part 8 §3/§4)

Connector credentials (Part 7 §2.2) are stored via SeaORM:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "merchant_acquirer_link")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub link_id: Uuid,
    pub connector_id: String,
    pub status: String,           // 'connected_untested' | 'active' | 'disabled'
    pub encrypted_config: Vec<u8>, // envelope-encrypted JSON blob (Part 8 §3 SEC-001)
    pub dek_wrapped: Vec<u8>,     // the DEK, wrapped by platform KEK
    pub created_at: DateTimeWithTimeZone,
}
```

No column in this entity ever holds a plaintext secret, satisfying CRED-001/ENC-003 structurally.

---

## 2. Redis — Caching & Ephemeral State Strategy

| Use | Key Pattern | TTL | Owning Service |
|---|---|---|---|
| Idempotency dedup (fast path, Part 5 §4.2 CONC-002) | `idem:{idempotency_key}` → payment_intent_id | 24h | `orchestration-service` |
| Hot routing policy cache | `routing_policy:active` → serialized policy | Invalidated on `RoutingPolicyActivated`, not purely TTL-based | `orchestration-service` |
| Session/permission cache | `perm:{principal_id}` → resolved ABAC policy set | 5 min or invalidated on `RoleAssigned` | `iam-service` |
| API Gateway rate-limit counters | `ratelimit:{endpoint}:{window}` | Sliding window, short TTL | `api-gateway` |
| Notification delivery dedup | `notif_sent:{notification_id}` | 7 days | `notification-service` |

- **REDIS-001**: Redis is treated as a performance/availability optimization layer only, never a system of record — every cache entry above has a durable Postgres (or event-store) source of truth it can be rebuilt from (Part 5 §4.2 CONC-002 principle applied platform-wide).

---

## 3. ClickHouse — Analytics Schema (Rust ClickHouse Driver)

### 3.1 Design Approach

Append-only, denormalized, wide event tables optimized for the analytical query patterns behind UC-070/UC-071 (unified dashboard, reconciliation export) and the AI Assistant's summary-document ingestion (Part 6 §3.1 ING-001).

```go
// analytics/model/payment_events.go — Go ClickHouse driver (not ORM — ClickHouse is append-only analytics)
type PaymentEvent struct {
    EventType         string    `ch:"event_type"`
    PaymentIntentID   string    `ch:"payment_intent_id"`
    AcquirerID        string    `ch:"acquirer_id"`
    CardScheme        string    `ch:"card_scheme"`
    Currency          string    `ch:"currency"`
    AmountMinorUnits  int64     `ch:"amount_minor_units"`
    DeclineReason     string    `ch:"decline_reason"`
    LatencyMs         uint32    `ch:"latency_ms"`
    OccurredAt        time.Time `ch:"occurred_at"`
    EventDate         time.Time `ch:"event_date"` // materialized toDate(occurred_at)
}
```

Note: ClickHouse uses a Rust native driver (e.g., `clickhouse-rs` or `clickhouse` crate) because ClickHouse is append-only analytics with no entity lifecycle management — it's a pure data sink for analytical queries. SeaORM is used for Postgres-based services that have entity CRUD lifecycle.

```sql
CREATE TABLE payment_events (
    event_type          LowCardinality(String),
    payment_intent_id   UUID,
    acquirer_id          LowCardinality(String),
    card_scheme          LowCardinality(String),
    currency             LowCardinality(String),
    amount_minor_units    Int64,
    decline_reason        LowCardinality(String),
    latency_ms            UInt32,
    occurred_at            DateTime64(3),
    event_date             Date MATERIALIZED toDate(occurred_at)
) ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (event_date, acquirer_id, card_scheme);
```

- **CH-001**: This table is populated by a dedicated NATS-consuming ingestion process within `analytics-service` (SVC-15) subscribing to the full domain event catalog (Part 3 §4), flattening each event into this wide-row shape — analytics never queries the transactional Postgres event stores directly, keeping the checkout-hot-path database isolated from heavy analytical query load (an explicit reliability requirement, not just a performance nicety).
- **CH-002**: Materialized views pre-aggregate common dashboard queries (e.g., hourly authorization-rate-per-acquirer rollups) so UC-070's dashboard doesn't scan raw event rows on every page load.

```sql
CREATE MATERIALIZED VIEW auth_rate_hourly_mv
ENGINE = SummingMergeTree
PARTITION BY toYYYYMM(hour)
ORDER BY (hour, acquirer_id, card_scheme)
AS
SELECT
    acquirer_id, card_scheme,
    toStartOfHour(occurred_at) AS hour,
    countIf(event_type = 'PaymentAuthorized') AS approved_count,
    countIf(event_type = 'PaymentFailed') AS declined_count
FROM payment_events
GROUP BY acquirer_id, card_scheme, hour;
```

This same `auth_rate_hourly_mv`-style rollup is exactly the "summary document" source referenced in Part 6 §3.1 ING-001 for the AI Assistant's ingestion path, and (H3) the baseline data source for GOAL-010's anomaly detection (Part 6 §7).

---

## 4. OpenSearch — Vector & Text Retrieval Index Design

### 4.1 Index Strategy

- **OS-001**: A single OpenSearch index (`rag_index`) is used for the RAG retrieval. The index is designed with strong field-level access control to ensure the operator can only query their own data.

### 4.2 Index Mapping (Representative)

```json
{
  "mappings": {
    "properties": {
      "chunk_id": { "type": "keyword" },
      "source_type": { "type": "keyword" },
      "source_id": { "type": "keyword" },
      "text_content": { "type": "text" },
      "text_content_ar": { "type": "text", "analyzer": "arabic" },
      "dense_vector": { "type": "knn_vector", "dimension": 1024 },
      "occurred_at": { "type": "date" },
      "citation_metadata": { "type": "object", "enabled": true }
    }
  }
}
```

- **OS-002**: `dense_vector` dimension (1024, representative of BGE-M3's embedding size — exact dimension to be confirmed against the deployed model variant in Part 6 finalization) supports approximate k-NN search; `text_content`/`text_content_ar` support BM25 sparse/lexical search, combined via hybrid search scoring (Part 6 §3.2 step 4) before the cross-encoder reranker narrows results.
- **OS-003**: `citation_metadata` carries exactly what's needed to render a validated citation (Part 6 §3.2 step 5, GRD-OUT-001) — source type (transaction/document/summary), source ID resolvable back to the owning service's record, and enough excerpt/location detail for a human to verify the claim.

---

## 5. MinIO — Object Storage Bucket Design

| Bucket | Contents | Encryption | Retention |
|---|---|---|---|
| `kyb-evidence` | Trade licenses, ID documents, proof of address | Per-object KMS-backed (Part 8 §4.2 ENC-004) | Per AUD-001 floor (Part 8) |
| `settlement-files` | Raw ingested settlement files (SFTP drops, scanned advices) | Per-object KMS-backed | Per AUD-001 floor |
| `exported-reports` | Reconciliation report exports (UC-071) | Per-object KMS-backed | Per AUD-001 floor (audit reproducibility, BR-071-1) |
| `document-uploads` | General operator-uploaded documents (AI Assistant ad hoc, PROC-06) | Per-object KMS-backed | Configurable, shorter default unless linked to a compliance/financial record |

- **MINIO-001**: Single-bucket design with path-based organization (e.g., `kyb-evidence/license-001.pdf`) is simpler for single-tenant deployment. Access control is enforced at the application layer.

---

## 6. Outbox Table Schema (Transactional Outbox)

### 6.1 Outbox Entity (SeaORM — Rust)

Every event-sourced service includes an outbox entity alongside its event store entity, written within the same transaction (Part 3 §9.2):

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "outbox")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub outbox_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: i16,
    pub payload: Vec<u8>,          // same protobuf-encoded EventEnvelope as event_store
    pub created_at: DateTimeWithTimeZone,
    pub published_at: Option<DateTimeWithTimeZone>,
}
```

- **DB-005**: The outbox entry is written in the SAME Postgres transaction as the `event_store` append (Part 3 OUTBOX-001). This guarantees that if the event is committed to Postgres, it will eventually be published to NATS — no events are silently lost.
- **DB-006**: The outbox relay process (Part 3 §9.2) marks entries as `published_at` after successful NATS publish with acknowledgment. Unpublished entries are retried with exponential backoff.
- **DB-007**: Outbox entries are purged after `published_at` + a configurable retention (default: 7 days) to prevent unbounded table growth. The retention must exceed the worst-case relay downtime window.

### 6.2 Event Store Archival

- **DB-008**: Event stores for aggregates in terminal states are archived to a cold-storage table after a configurable retention period (Part 3 ARCH-001, default 90 days). The archive uses the same SeaORM entity as `event_store` but with an additional `archived_at` field:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "event_store_archive")]
pub struct Model {
    // identical fields to event_store Model, plus:
    pub archived_at: DateTimeWithTimeZone,
}
```

- **DB-009**: Archival is performed by a background job that moves event streams for terminal-state aggregates from `event_store` to `event_store_archive`. The job respects the legal retention floor (Part 8 AUD-001) — no events are archived or deleted before the floor expires.
- **DB-010**: Archived event streams can be rehydrated on demand (for compliance audit or investigation) by moving them back to `event_store` and rebuilding projections.

---

## 8. Schema Migration — Maker/Checker

- **MIG-MKCK-001**: All database schema migrations follow the Maker/Checker pattern:
  - **Maker**: Developer creates a migration file (forward + rollback) in a feature branch
  - **Checker 1**: Tech Lead reviews the migration for correctness, data integrity, and business logic
  - **Checker 2**: DBA reviews the migration for performance impact, locking behavior, and index strategy
  - Both Checkers must approve before the migration is merged and executed in production

- **MIG-MKCK-002**: Migration execution in production requires:
  1. Migration file approved by both Checkers (recorded in `change_history`)
  2. Migration tested against a staging database clone
  3. Migration executed during a maintenance window (for breaking changes) or online (for backward-compatible changes)
  4. Post-migration verification (automated health checks + manual spot-check)
  5. Rollback procedure documented and tested before execution

- **MIG-MKCK-003**: Migration history is tracked in a dedicated `migration_history` table (auto-managed by SeaORM/Ent migration framework):
  - Migration version number
  - Status (pending → applied → verified → rolled_back)
  - Maker (who created)
  - Checker(s) (who approved)
  - Applied at timestamp
  - Rollback available (boolean)
  - Duration (how long the migration took)

### 9.1 Connection Pool Management

- **POOL-001**: Each service's Postgres connection pool is managed via PgBouncer (or the service's built-in connection pooler if using async Rust with `sqlx`) with the following configuration:
  - Pool size per service: configurable, default 20 connections for event-sourced services, 10 for supporting services
  - Connection timeout: 5 seconds
  - Idle timeout: 300 seconds
  - Max lifetime: 1800 seconds (prevents stale connections)

- **POOL-002**: The connection pool is shared across all application processes (authorization is enforced at the query level via ABAC (Part 8), not at the connection level).

- **POOL-003**: Read-heavy services (`analytics-service`, `ai-assistant-service`) use read-replica Postgres connections for query operations, with the primary reserved for writes. Read-replica lag is monitored as a first-class metric (Part 11 OBS-004).

---

## 10. Cross-Store Consistency Notes

- **XSTORE-001**: Because different stores serve different consistency roles (Postgres event store = strong/source-of-truth; ClickHouse/OpenSearch = eventually consistent projections), every dashboard/report/AI answer surface must be able to express "as of" freshness (Part 3 §7) — this is a UI/API contract requirement carried into Part 10, not just an internal implementation detail to hide from users.
- **XSTORE-002**: NATS JetStream consumer offsets (Part 4 §4.2) are the mechanism by which ClickHouse/OpenSearch projections track "how far behind" they are relative to the Postgres event stores; monitoring this lag is an operational metric (Part 11) directly tied to XSTORE-001's freshness disclosure.

---

## 11. Traceability

| Requirement | Realized By |
|---|---|
| BIZ-040 (immutable audit) | §1.1 event store design, §1.3 DB-004 (DB-privilege-enforced append-only) |
| BIZ-041 (data residency) | All stores deployed UAE-region (deployment detail, Part 11) |
| Part 3 INV-06 (idempotent settlement ingestion) | Implicit via `event_id`/checksum uniqueness constraints (§1.1) |
| Part 5 §4.2 CONC-001/002 | §1.1 DB-001 optimistic concurrency, §2 REDIS-001 |
| Part 6 AI-P-002 (structural tenant isolation for RAG) | §4.1 OS-001 |
| Part 7 CRED-001/002 | §1.4 encrypted config storage |
| Part 8 SEC-001, ENC-003/004 | §1.4, §5 MinIO encryption |
| Part 8 AUD-003 (immutability) | §1.3 DB-004 |
| Transactional Outbox (event publish reliability) | §6.1 DB-005 through DB-007 |
| Event Store Archival (lifecycle management) | §6.2 DB-008 through DB-010 |
| ClickHouse analytics schema | §3 CH-001, CH-002 |
| Connection pool management | §9 POOL-001 through POOL-003 |

---

## 12. Open Items Carried Forward

- **OQ-020**: Confirm exact BGE-M3 embedding dimension for the deployed model variant/quantization (§4.2 OS-002 placeholder of 1024).
- **OQ-021**: Finalize event-store retention/archival policy — whether older event partitions are archived to MinIO (cold storage).
- **OQ-022**: Resolve OQ-010 (Part 4) — shared vs. separate Postgres instances for `invoice-service`/`payment-link-service`.
- **OQ-046**: Finalize outbox relay polling interval (§6.1 DB-006) vs. CDC (Debezium) trade-offs once event volume justifies the infrastructure complexity.
- **OQ-047**: Confirm PgBouncer vs. built-in connection pooler choice (§9 POOL-001) against the chosen async Rust runtime and sqlx configuration.

---

*End of Part 9. Proceed to Part 10: APIs & gRPC Contracts.*
