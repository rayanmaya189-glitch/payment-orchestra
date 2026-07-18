# Software Requirements Specification
## Multi-Tenant AI-Native Payment Orchestration Platform (UAE-First, Multi-Country Ready)

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
| Golden Rule | Every table/index is designed for single-tenant deployment — no tenant_id columns needed. Access control is enforced at the application layer via RBAC/ABAC (Part 8), not at the data layer. |

---

## 1. PostgreSQL — Event Store Design (Event-Sourced Contexts)

### 1.1 Shared Event Store Schema Pattern

Each event-sourced service (BC-05 `orchestration-service`, BC-08 `subscription-service`, BC-09 `reconciliation-service`, BC-10 `dispute-service`) owns its own Postgres database using the same event-store table shape, so the pattern is documented once here rather than four times.

```sql
CREATE TABLE event_store (
    aggregate_type       TEXT        NOT NULL,
    aggregate_id         UUID        NOT NULL,
    event_sequence       BIGINT      NOT NULL,   -- per-aggregate monotonic version
    event_id             UUID        NOT NULL,
    event_type           TEXT        NOT NULL,
    event_version        SMALLINT    NOT NULL,
    occurred_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_type           TEXT        NOT NULL,   -- 'user' | 'api_key' | 'system'
    actor_id             UUID        NULL,
    causation_id         UUID        NULL,
    correlation_id       UUID        NOT NULL,
    payload              BYTEA       NOT NULL,   -- protobuf-encoded (Part 10)
    PRIMARY KEY (aggregate_type, aggregate_id, event_sequence)
);

CREATE UNIQUE INDEX event_store_event_id_uq ON event_store (event_id);
CREATE INDEX event_store_correlation_idx ON event_store (correlation_id);
CREATE INDEX event_store_occurred_at_idx ON event_store (occurred_at);
```

- **DB-001 (Optimistic concurrency)**: Appends specify `expected_event_sequence`; the insert is conditioned (application-level check-then-insert within a transaction, or a Postgres `EXCLUDE`/unique-constraint-based guard on `(aggregate_type, aggregate_id, event_sequence)`) so a concurrent writer's stale-sequence append fails and must reload+retry (Part 5 §4.2 CONC-001).
- **DB-002 (Payload encoding)**: `payload` is protobuf, not JSON, for compactness and schema evolution discipline (Part 10 defines `.proto` schemas per event type, with explicit backward-compatibility rules — additive fields only within a version, breaking changes bump `event_version`).
- **DB-003 (Snapshotting)**: For aggregates with long event streams (e.g., a `Subscription` that has renewed monthly for years), a `aggregate_snapshot` table stores periodic materialized state (every N events or T time) to bound replay cost:

```sql
CREATE TABLE aggregate_snapshot (
    aggregate_type   TEXT NOT NULL,
    aggregate_id     UUID NOT NULL,
    as_of_sequence   BIGINT NOT NULL,
    state            BYTEA NOT NULL,   -- protobuf-encoded materialized aggregate state
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (aggregate_type, aggregate_id, as_of_sequence)
);
```

### 1.2 Read-Model (Projection) Tables

Projections are plain, indexed-for-query Postgres tables (or ClickHouse for heavy analytics, §3), rebuilt from the event stream and kept eventually consistent (Part 3 §7 CQRS). Representative example — the reconciliation exception queue (UC-041):

```sql
CREATE TABLE reconciliation_exception_projection (
    exception_id          UUID NOT NULL,
    settlement_record_id   UUID NOT NULL,
    payment_intent_id      UUID NULL,       -- null until/unless manually matched
    amount_minor_units     BIGINT NOT NULL,
    currency               CHAR(3) NOT NULL,
    status                 TEXT NOT NULL,   -- 'unmatched' | 'resolved' | 'flagged_discrepancy'
    detected_at            TIMESTAMPTZ NOT NULL,
    resolved_at            TIMESTAMPTZ NULL,
    resolved_by_actor_id   UUID NULL,
    PRIMARY KEY (exception_id)
);
CREATE INDEX recon_exc_status_idx ON reconciliation_exception_projection (status, detected_at);
```

### 1.3 Non-Event-Sourced Service Schemas (Representative)

For BC-01/02/03/13/14 (Tier-2-audited per Part 8 §5.1), tables are conventional normalized CRUD-plus-audit-log:

```sql
-- operator-management (SVC-01)
CREATE TABLE operator (
    operator_id     UUID PRIMARY KEY,
    legal_name      TEXT NOT NULL,
    trade_license_no TEXT NOT NULL,
    country         CHAR(2) NOT NULL DEFAULT 'AE',
    status          TEXT NOT NULL,   -- Pending | Active-Unverified | Active-Verified | Suspended | Expired-Unverified
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- iam-service (SVC-02)
CREATE TABLE principal (
    principal_id    UUID PRIMARY KEY,
    principal_type  TEXT NOT NULL,  -- 'human' | 'api_key' | 'service'
    email           TEXT NULL,
    password_hash   TEXT NULL,      -- argon2id
    mfa_enrolled    BOOLEAN NOT NULL DEFAULT false,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE role_assignment (
    principal_id    UUID NOT NULL,
    role            TEXT NOT NULL,
    abac_conditions JSONB NOT NULL DEFAULT '{}',  -- e.g., {"max_refund_amount_minor": 500000}
    PRIMARY KEY (principal_id, role)
);

-- Tier 2 audit log
CREATE TABLE audit_log (
    audit_id        UUID NOT NULL,
    actor_id        UUID NULL,
    action          TEXT NOT NULL,
    before_state     JSONB NULL,
    after_state      JSONB NULL,
    occurred_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    source_ip        INET NULL,
    PRIMARY KEY (audit_id)
);
```

- **DB-004**: `audit_log` is append-only at the database-privilege level — the application's database role for these services is granted `INSERT`/`SELECT` only on `audit_log`, with no `UPDATE`/`DELETE` grant at all (Part 8 §5.3 AUD-003 enforced at the DB-permission layer, not merely by application code discipline).

### 1.4 Encrypted Field Storage (Ties to Part 8 §3/§4)

Connector credentials (Part 7 §2.2) are stored as:

```sql
CREATE TABLE merchant_acquirer_link (
    link_id               UUID NOT NULL,
    connector_id           TEXT NOT NULL,
    status                 TEXT NOT NULL,   -- Connected-Untested | Active | Disabled
    encrypted_config       BYTEA NOT NULL,   -- envelope-encrypted JSON blob (Part 8 §3 SEC-001)
    dek_wrapped            BYTEA NOT NULL,   -- the DEK, wrapped by platform KEK
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (link_id)
);
```

No column in this table ever holds a plaintext secret, satisfying CRED-001/ENC-003 structurally.

---

## 2. Redis — Caching & Ephemeral State Strategy

| Use | Key Pattern | TTL | Owning Service |
|---|---|---|---|
| Idempotency dedup (fast path, Part 5 §4.2 CONC-002) | `idem:{idempotency_key}` → payment_intent_id | 24h | `orchestration-service` |
| Hot routing policy cache | `routing_policy:active` → serialized policy | Invalidated on `RoutingPolicyActivated`, not purely TTL-based | `orchestration-service` |
| Session/permission cache | `perm:{principal_id}` → resolved role/ABAC set | 5 min or invalidated on `RoleAssigned` | `iam-service` |
| API Gateway rate-limit counters | `ratelimit:{endpoint}:{window}` | Sliding window, short TTL | `api-gateway` |
| Notification delivery dedup | `notif_sent:{notification_id}` | 7 days | `notification-service` |

- **REDIS-001**: Redis is treated as a performance/availability optimization layer only, never a system of record — every cache entry above has a durable Postgres (or event-store) source of truth it can be rebuilt from (Part 5 §4.2 CONC-002 principle applied platform-wide).

---

## 3. ClickHouse — Analytics Schema

### 3.1 Design Approach

Append-only, denormalized, wide event tables optimized for the analytical query patterns behind UC-070/UC-071 (unified dashboard, reconciliation export) and the AI Assistant's summary-document ingestion (Part 6 §3.1 ING-001).

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

### 6.1 Outbox Table Pattern

Every event-sourced service includes an `outbox` table alongside its `event_store` table, written within the same transaction (Part 3 §9.2):

```sql
CREATE TABLE outbox (
    outbox_id       UUID NOT NULL,
    aggregate_type  TEXT NOT NULL,
    aggregate_id    UUID NOT NULL,
    event_type      TEXT NOT NULL,
    event_version   SMALLINT NOT NULL,
    payload         BYTEA NOT NULL,     -- same protobuf-encoded EventEnvelope as event_store
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at    TIMESTAMPTZ NULL,   -- set by the relay process after NATS publish confirms
    PRIMARY KEY (outbox_id)
);

CREATE INDEX outbox_unpublished_idx ON outbox (created_at) WHERE published_at IS NULL;
```

- **DB-005**: The outbox entry is written in the SAME Postgres transaction as the `event_store` append (Part 3 OUTBOX-001). This guarantees that if the event is committed to Postgres, it will eventually be published to NATS — no events are silently lost.
- **DB-006**: The outbox relay process (Part 3 §9.2) marks entries as `published_at` after successful NATS publish with acknowledgment. Unpublished entries are retried with exponential backoff.
- **DB-007**: Outbox entries are purged after `published_at` + a configurable retention (default: 7 days) to prevent unbounded table growth. The retention must exceed the worst-case relay downtime window.

### 6.2 Event Store Archival

- **DB-008**: Event stores for aggregates in terminal states are archived to a cold-storage table after a configurable retention period (Part 3 ARCH-001, default 90 days):

```sql
CREATE TABLE event_store_archive (
    -- identical schema to event_store (Part 9 §1.1)
    aggregate_type       TEXT        NOT NULL,
    aggregate_id         UUID        NOT NULL,
    event_sequence       BIGINT      NOT NULL,
    event_id             UUID        NOT NULL,
    event_type           TEXT        NOT NULL,
    event_version        SMALLINT    NOT NULL,
    occurred_at          TIMESTAMPTZ NOT NULL,
    actor_type           TEXT        NOT NULL,
    actor_id             UUID        NULL,
    causation_id         UUID        NULL,
    correlation_id       UUID        NOT NULL,
    payload              BYTEA       NOT NULL,
    archived_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (aggregate_type, aggregate_id, event_sequence)
);
```

- **DB-009**: Archival is performed by a background job that moves event streams for terminal-state aggregates from `event_store` to `event_store_archive`. The job respects the legal retention floor (Part 8 AUD-001) — no events are archived or deleted before the floor expires.
- **DB-010**: Archived event streams can be rehydrated on demand (for compliance audit or investigation) by moving them back to `event_store` and rebuilding projections.

---

## 8. Connection Pool Management

### 9.1 PgBouncer / Built-In Connection Pooling

- **POOL-001**: Each service's Postgres connection pool is managed via PgBouncer (or the service's built-in connection pooler if using async Rust with `sqlx`) with the following configuration:
  - Pool size per service: configurable, default 20 connections for event-sourced services, 10 for supporting services
  - Connection timeout: 5 seconds
  - Idle timeout: 300 seconds
  - Max lifetime: 1800 seconds (prevents stale connections)

- **POOL-002**: The connection pool is shared across all application processes (authorization is enforced at the query level via RBAC/ABAC (Part 8), not at the connection level).

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
| ClickHouse tenant isolation (partition-level) | §8 CH-003, CH-004 |
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
