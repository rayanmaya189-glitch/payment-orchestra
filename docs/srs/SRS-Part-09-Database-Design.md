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

This same `auth_rate_hourly_mv`-style rollup is exactly the "summary document" source referenced in Part 6 §3.1 ING-001 for the AI Assistant's ingestion path, and (Phase 3) the baseline data source for GOAL-010's anomaly detection (Part 6 §7).

---

## 4. OpenSearch — Vector & Text Retrieval Index Design

### 4.1 Index Lifecycle Management

- **OS-ILM-001**: OpenSearch indices follow an Index Lifecycle Management (ILM) policy:
  - **Hot phase**: Active writes and searches. Shard count sized for expected query volume.
  - **Warm phase**: After 30 days, indices are force-merged to reduce segment count and improve search performance. No new writes.
  - **Cold phase**: After 90 days, indices are moved to cold storage (cheaper SSD/HDD) for historical queries.
  - **Delete phase**: After 365 days, indices are deleted (unless compliance retention requires longer).

- **OS-ILM-002**: Shard allocation strategy: each index has a configurable number of primary shards (default: 1 for small indices, 3 for large indices) and replica shards (default: 1 for production, 0 for development). Replica shards provide read scaling and fault tolerance.

- **OS-ILM-003**: Index optimization: warm-phase indices are force-merged to a single segment per shard to reduce file descriptor usage and improve query performance. This is a read-only operation that improves search speed at the cost of write capability.

## 5. ClickHouse — Replication Strategy

### 5.1 Replication

- **CH-REP-001**: ClickHouse tables use `ReplicatedMergeTree` engine family for high availability. Each table has at least 2 replicas across different availability zones.
- **CH-REP-002**: Replication is asynchronous — writes are acknowledged to the writer before replicas are updated. This provides eventual consistency for analytics queries but ensures no data loss on single-replica failure.
- **CH-REP-003**: Materialized views (Part 9 §3.1 CH-002) are also replicated, ensuring consistent aggregated data across replicas.

### 5.2 Backup and Restore

- **CH-BACK-001**: ClickHouse backups are performed daily using `clickhouse-backup` tool, storing backups to MinIO (S3-compatible) with 30-day retention.
- **CH-BACK-002**: Restore procedure: restore from backup into a fresh ClickHouse cluster, then replay NATS JetStream retained events to rebuild the projection from the last consistent state.

## 6. MinIO — Object Storage Lifecycle Policies

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

### 5.1 Bucket Lifecycle Policies

- **MINIO-LC-001**: Each bucket has lifecycle rules:
  - **`kyb-evidence`**: Objects transition to IA (infrequent access) storage class after 90 days. Objects are retained for 7 years minimum (compliance, Part 8 AUD-001). No automatic deletion.
  - **`settlement-files`**: Objects transition to IA after 30 days. Retained for 5 years minimum (financial record retention). Automatic deletion after retention expires.
  - **`exported-reports`**: Objects transition to IA after 30 days. Retained for 2 years minimum (audit reproducibility, BR-071-1).
  - **`document-uploads`**: Temporary uploads expire after 7 days unless linked to a compliance/financial record. Linked documents follow the compliance retention schedule.

- **MINIO-LC-002**: Bucket versioning is enabled for `kyb-evidence` and `settlement-files` buckets to prevent accidental overwrites and enable point-in-time recovery. Versioning is not enabled for `document-uploads` (temporary data).

- **MINIO-LC-003**: Cross-region replication is configured for `kyb-evidence` and `settlement-files` buckets to a secondary MinIO cluster in a different availability zone, providing disaster recovery for compliance-critical documents.

## 7. Redis — High Availability Strategy

### 7.1 Redis HA Configuration

- **REDIS-HA-001**: Redis is deployed in Redis Sentinel mode (not Redis Cluster for initial launch) with:
  - 1 primary instance (handles all writes)
  - 2 sentinel instances (monitor primary health, coordinate failover)
  - Automatic failover: if primary fails, sentinel promotes a replica within 10 seconds
  - Persistence: AOF (Append-Only File) with `everysec` fsync for data durability

- **REDIS-HA-002**: Redis memory management:
  - Max memory: configurable per deployment (default: 2GB)
  - Eviction policy: `allkeys-lru` (least recently used) — since Redis is a cache layer only (REDIS-001), evicting stale entries is acceptable
  - Memory fragmentation monitoring: alert if `mem_fragmentation_ratio` > 1.5

- **REDIS-HA-003**: Connection pooling: each service maintains its own Redis connection pool (default: 10 connections, 5-second timeout, 300-second idle timeout). Connection pool exhaustion is monitored and alerts raised if pool utilization exceeds 80%.

- **REDIS-HA-004**: Redis data is backed up daily via RDB snapshots to MinIO. Recovery from backup restores the cache layer — no data loss impact since Redis is not a system of record (REDIS-001).

## 8. Connection Pool Management

### 6.1 Outbox Entity (SeaORM — Rust)

Every event-sourced service includes an outbox entity alongside its event store entity, written within the same transaction (Part 3 §9.2):

#### 6.1.1 Outbox Table Partitioning

- **DB-014**: The outbox table is partitioned by `created_at` (monthly partitions) to prevent unbounded growth from degrading relay performance. Old partitions (published and older than 7 days) are dropped automatically by a background job.
- **DB-015**: Outbox entries are purged after `published_at` + a configurable retention (default: 7 days) to prevent unbounded table growth. The retention must exceed the worst-case relay downtime window.
- **DB-016**: Outbox relay performance is monitored: if the relay's polling interval exceeds 2x the target (default: 2 seconds), an alert is raised. The relay exposes a `/healthz` endpoint (Part 4 HEALTH-001) and a lag metric (`outbox_relay_lag`).

#### 6.1.2 Outbox Relay Monitoring

- **OUTBOX-MON-001**: The outbox relay exposes metrics:
  - `outbox_relay_lag` — number of unpublished entries
  - `outbox_relay_publish_latency_ms` — time to publish a batch
  - `outbox_relay_errors_total` — count of failed publish attempts
- **OUTBOX-MON-002**: Alerts are raised when:
  - `outbox_relay_lag` > 100 (warning) or > 1000 (critical)
  - `outbox_relay_errors_total` increases by > 10 in 5 minutes
  - Relay is not heartbeat-positive for > 30 seconds

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

### 9.2 Connection Pool Sizing Guidelines

- **POOL-SIZE-001**: Connection pool sizing follows the formula: `pool_size = (number_of_cpu_cores * 2) + disk_spindles`. For typical cloud instances (4 vCPU, SSD): pool size = 9 connections.
- **POOL-SIZE-002**: Event-sourced services (orchestration, reconciliation, subscription, dispute) use larger pools (20 connections) due to higher write throughput during peak checkout periods.
- **POOL-SIZE-003**: Connection pool exhaustion is monitored via PgBouncer stats (`cl_active`, `cl_waiting`). When `cl_waiting` > 0 for more than 5 seconds, an alert is raised. When `cl_waiting` > 10, the service is flagged as degraded.
- **POOL-SIZE-004**: Connection pool metrics are exported to Prometheus and included in the service's RED metrics (Part 11 OBS-003).

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

## 12. Gap Analysis Additions — Database Security & Data Lifecycle

### 12.1 Row-Level Security (RLS) Policy Examples

**DB-011-EX1**: Representative RLS policy for `event_store` (orchestration-service):

```sql
-- Enable RLS on the event_store table
ALTER TABLE event_store ENABLE ROW LEVEL SECURITY;

-- Policy: principal can only read events for their operator's aggregates
CREATE POLICY event_store_isolation ON event_store
  FOR SELECT
  USING (
    aggregate_id IN (
      SELECT pi.payment_intent_id
      FROM payment_intent pi
      WHERE pi.operator_id = current_setting('app.current_operator_id')::uuid
    )
  );

-- Policy: only the orchestration-service can insert events
CREATE POLICY event_store_insert ON event_store
  FOR INSERT
  WITH CHECK (current_setting('app.service_role') = 'orchestration-service');
```

**DB-011-EX2**: Representative RLS policy for `reconciliation_exception_projection`:

```sql
ALTER TABLE reconciliation_exception_projection ENABLE ROW LEVEL SECURITY;

CREATE POLICY reconciliation_exception_isolation ON reconciliation_exception_projection
  FOR ALL
  USING (
    operator_id = current_setting('app.current_operator_id')::uuid
  );
```

**DB-011-EX3**: Postgres role setup:

```sql
-- Service-specific roles (no superuser for application connections)
CREATE ROLE orchestration_service_role;
CREATE ROLE reconciliation_service_role;
CREATE ROLE iam_service_role;

-- Grant minimal privileges
GRANT USAGE ON SCHEMA orchestration TO orchestration_service_role;
GRANT SELECT, INSERT ON event_store TO orchestration_service_role;
GRANT SELECT, INSERT ON outbox TO orchestration_service_role;
-- No UPDATE/DELETE on event_store (append-only)

-- Set session variables for RLS
SET app.service_role = 'orchestration-service';
SET app.current_operator_id = '<uuid>';
```

### 12.2 ClickHouse Security Configuration

**CH-SEC-001**: ClickHouse user per service with read-only access to required tables:

```sql
-- Create analytics-service user (read-only)
CREATE USER analytics_service IDENTIFIED BY '<password>';
GRANT SELECT ON payment_events TO analytics_service;
GRANT SELECT ON auth_rate_hourly_mv TO analytics_service;

-- Create ai-assistant-service user (read-only, specific tables)
CREATE USER ai_assistant_service IDENTIFIED BY '<password>';
GRANT SELECT ON payment_events TO ai_assistant_service;
```

**CH-SEC-002**: ClickHouse TLS for client connections enabled via `<openSSL>` server configuration.

**CH-SEC-003**: Network policy restricting ClickHouse access to only `analytics-service` and `ai-assistant-service`.

**CH-SEC-004**: ClickHouse query logging enabled for audit trail (`system.query_log`).

### 12.3 OpenSearch Security Configuration

**OS-SEC-001**: OpenSearch security plugin enabled with: TLS for all inter-node and client connections, basic auth for application connections, index-level security policies.

**OS-SEC-002**: Network policy restricting OpenSearch access to `ai-assistant-service` only.

**OS-SEC-003**: OpenSearch audit logging enabled for all search and index operations.

**OS-SEC-004**: Resolve OQ-042 (per-tenant indices vs. shared index) with security as the primary decision criterion. Recommended: per-tenant index routing for single-tenant deployment (simpler isolation), migrating to shared index with document-level security if multi-tenant is needed.

### 12.4 MinIO Security Configuration

**MINIO-SEC-001**: MinIO deployed with TLS and access key/secret authentication. Default `minioadmin:minioadmin` credentials rotated on first deployment.

**MINIO-SEC-002**: Per-service IAM policies:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": { "AWS": ["arn:aws:iam::document-service"] },
      "Action": ["s3:PutObject", "s3:GetObject", "s3:DeleteObject"],
      "Resource": ["arn:aws:s3:::document-uploads/*"]
    },
    {
      "Effect": "Allow",
      "Principal": { "AWS": ["arn:aws:iam::compliance-service"] },
      "Action": ["s3:GetObject"],
      "Resource": ["arn:aws:s3:::kyb-evidence/*"]
    }
  ]
}
```

**MINIO-SEC-003**: Network policy restricting MinIO access to only `document-service`, `compliance-service`, `reconciliation-service`, and `analytics-service`.

**MINIO-SEC-004**: MinIO audit logging to SIEM for all access operations.

### 12.5 Data Masking for Non-Production Environments

**MASK-001**: A `DataMaskingService` runs as part of the database snapshot/copy process used to create staging from production. It replaces:
- Real acquirer credentials with sandbox equivalents
- Real KYB document references with synthetic ones
- Real card tokens with synthetic tokens
- Real email addresses with synthetic addresses

**MASK-002**: All staging/test environments use separate MinIO buckets and separate database instances — never shared with production.

**MASK-003**: A CI gate scans database fixtures for PII patterns (email regex, card number patterns, API key patterns) and fails the build if real data is detected.

### 12.6 Database Connection Security

**DB-CONN-001**: All Postgres connections use `sslmode=verify-full` (not just `require`). The application rejects connections where TLS negotiation fails or certificate verification fails.

**DB-CONN-002**: Connection-string validation in CI: no `sslmode=disable` or `sslmode=allow` permitted in any environment configuration.

**DB-CONN-003**: Database IP allowlisting via `pg_hba.conf` or cloud security groups, restricting connections to known service IPs only.

### 12.7 Automated Data Retention Enforcement

**RETAIN-001**: A `DataRetentionEnforcer` background job per service:
1. Queries for aggregates in terminal states older than the configured retention period
2. For each: moves event stream from `event_store` to `event_store_archive`, deletes snapshot data beyond the retention floor, marks read-model projections as archived (but retains them)
3. Logs each archival action in a `retention_audit_log` table: `aggregate_id`, `events_archived_count`, `retention_policy_applied`, `archived_at`, `legal_retention_floor_checked`
4. Excludes aggregates with pending charges/disputes from archival regardless of age

**RETAIN-002**: The `retention_audit_log` is append-only and retained for 7 years (compliance).

### 12.8 Gap: New Table Schemas (Settlement Timing, Fee Variance, Tokens, Webhooks)

#### Settlement Expectation Table (Gap: T+N Tracking)

```sql
CREATE TABLE settlement_expectation (
    expectation_id UUID PRIMARY KEY,
    payment_intent_id UUID NOT NULL,
    acquirer_link_id UUID NOT NULL,
    expected_settlement_date DATE NOT NULL,
    settlement_cycle VARCHAR(20) NOT NULL,  -- 'same_day' | 'next_day' | 'two_days' | 'three_days' | 'weekly'
    status VARCHAR(20) NOT NULL,            -- 'pending' | 'settled' | 'overdue' | 'adjusted'
    settled_amount_minor_units BIGINT,
    settled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_settlement_expectation_status ON settlement_expectation(status);
CREATE INDEX idx_settlement_expectation_expected_date ON settlement_expectation(expected_settlement_date);
CREATE INDEX idx_settlement_expectation_payment_intent ON settlement_expectation(payment_intent_id);
```

#### Fee Variance Table (Gap: Fee Reconciliation)

```sql
CREATE TABLE fee_variance (
    variance_id UUID PRIMARY KEY,
    payment_intent_id UUID NOT NULL,
    acquirer_link_id UUID NOT NULL,
    estimated_fee_minor_units BIGINT NOT NULL,
    actual_fee_minor_units BIGINT NOT NULL,
    variance_amount_minor_units BIGINT NOT NULL,
    variance_percent DOUBLE PRECISION NOT NULL,
    is_within_tolerance BOOLEAN NOT NULL,
    tolerance_threshold_percent DOUBLE PRECISION NOT NULL DEFAULT 5.0,
    status VARCHAR(20) NOT NULL,            -- 'within_tolerance' | 'variance_detected' | 'disputed' | 'resolved'
    detected_at TIMESTAMPTZ NOT NULL,
    resolved_at TIMESTAMPTZ,
    resolution_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fee_variance_status ON fee_variance(status);
CREATE INDEX idx_fee_variance_payment_intent ON fee_variance(payment_intent_id);
```

#### Payment Method Token Table (Gap: Token Lifecycle)

```sql
CREATE TABLE payment_method_token (
    token_id UUID PRIMARY KEY,
    operator_id UUID NOT NULL,
    payment_method_type VARCHAR(20) NOT NULL,
    last_four VARCHAR(4) NOT NULL,
    card_brand VARCHAR(20),
    expiry_month INTEGER,
    expiry_year INTEGER,
    token_status VARCHAR(20) NOT NULL,      -- 'active' | 'expired' | 'revoked'
    acquirer_link_id UUID NOT NULL,
    acquirer_token_reference VARCHAR(255) NOT NULL,
    encrypted_token BYTEA NOT NULL,         -- envelope-encrypted
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    revocation_reason TEXT
);

CREATE INDEX idx_payment_method_token_operator ON payment_method_token(operator_id);
CREATE INDEX idx_payment_method_token_acquirer_link ON payment_method_token(acquirer_link_id);
CREATE INDEX idx_payment_method_token_status ON payment_method_token(token_status);
CREATE UNIQUE INDEX idx_payment_method_token_acquirer_ref ON payment_method_token(acquirer_token_reference);
```

#### Webhook Subscription Table (Gap: Outbound Webhooks)

```sql
CREATE TABLE webhook_subscription (
    subscription_id UUID PRIMARY KEY,
    operator_id UUID NOT NULL,
    url TEXT NOT NULL,                      -- HTTPS only
    event_types JSONB NOT NULL,             -- array of event type strings
    secret_hash BYTEA NOT NULL,             -- argon2id hash
    status VARCHAR(20) NOT NULL,            -- 'active' | 'disabled'
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_webhook_subscription_operator ON webhook_subscription(operator_id);
CREATE INDEX idx_webhook_subscription_status ON webhook_subscription(status);
```

#### Webhook Delivery Table (Gap: Outbound Webhooks)

```sql
CREATE TABLE webhook_delivery (
    delivery_id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload TEXT NOT NULL,
    signature VARCHAR(100) NOT NULL,        -- HMAC-SHA256
    status VARCHAR(20) NOT NULL,            -- 'pending' | 'delivered' | 'failed' | 'permanently_failed'
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMPTZ,
    next_retry_at TIMESTAMPTZ,
    response_status_code INTEGER,
    response_body TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_delivery_subscription ON webhook_delivery(subscription_id);
CREATE INDEX idx_webhook_delivery_status ON webhook_delivery(status);
CREATE INDEX idx_webhook_delivery_next_retry ON webhook_delivery(next_retry_at) WHERE status = 'pending';
```

---

## 13. Open Items Carried Forward

- **OQ-020**: Confirm exact BGE-M3 embedding dimension for the deployed model variant/quantization (§4.2 OS-002 placeholder of 1024).
- **OQ-021**: Finalize event-store retention/archival policy — whether older event partitions are archived to MinIO (cold storage).
- **OQ-022**: Resolve OQ-010 (Part 4) — shared vs. separate Postgres instances for `invoice-service`/`payment-link-service`.
- **OQ-046**: Finalize outbox relay polling interval (§6.1 DB-006) vs. CDC (Debezium) trade-offs once event volume justifies the infrastructure complexity.
- **OQ-047**: Confirm PgBouncer vs. built-in connection pooler choice (§9 POOL-001) against the chosen async Rust runtime and sqlx configuration.

---

*End of Part 9. Proceed to Part 10: APIs & gRPC Contracts.*
