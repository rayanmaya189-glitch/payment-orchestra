# 19 — Infrastructure & Cross-Cutting Concerns

n> **Architecture Context**: This module runs within the modular monolith alongside all other modules. All inter-module communication uses in-process gRPC (synchronous) or in-process NATS channels (asynchronous). The module boundaries defined here can be extracted into separate microservices in a future architecture evolution if scaling requires it.
Missing from all service-specific backend docs. Covers: outbox, health checks, graceful shutdown, leader election, feature flags, logging, connection pools, degraded modes, secrets, encryption, audit, SSRF, and operational hardening.

---

## 1. Transactional Outbox Pattern (Part 3 §9.2)

Every event-sourced service writes events to the `outbox` table in the SAME Postgres transaction as the aggregate state change.

**Outbox Entity (SeaORM)**:

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
    pub payload: Vec<u8>,            // protobuf-encoded event payload
    pub created_at: DateTimeWithTimeZone,
    pub published_at: Option<DateTimeWithTimeZone>,
}
```

**Relay Process**: Dedicated relay polls unpublished entries, publishes to NATS, marks published. Crash recovery resumes from last unconfirmed publish.

**OUTBOX-001**: Events are published to NATS if and only if the corresponding aggregate state change commits to Postgres. No silent event loss.

---

## 2. Health Check Endpoints (Part 4 §6.3)

Every service exposes on port 8081:

```rust
// GET /healthz — Liveness (HTTP 200 if process alive)
// GET /readyz — Readiness (HTTP 200 if can accept traffic)
//   Checks: Postgres ping, Redis PING, event store writable
// GET /startupz — Startup (HTTP 200 after initialization complete)
//   AI services: 120s timeout. Others: 10s.
// GET /healthz/deep — Deep health (authenticated or internal-only)
//   Returns: all subsystem health, latency, error states
```

**HEALTH-005**: Health endpoints never expose sensitive info (no IPs, connection strings, version details).

---

## 3. Graceful Shutdown (Part 4 §6.2)

Every service implements on SIGTERM:

```
1. Stop accepting new requests (deregister from load balancer)
2. Complete in-flight requests (drain timeout: 15s checkout-critical, 60s others)
3. Flush outbox entries
4. Release leader election locks
5. Close database connections
6. Exit with code 0
```

**SHUTDOWN-003**: K8s pod lifecycle hooks (`preStop` + `terminationGracePeriodSeconds`) match service drain timeout.

---

## 4. Leader Election (Part 4 §6.1)

Redis SETNX-based distributed locks with TTL:

```rust
pub struct LeaderElection {
    redis: RedisPool,
    job_type: String,
    ttl: Duration, // default: 30s (checkout), 60s (settlement poll)
}

impl LeaderElection {
    pub async fn try_acquire(&self) -> Result<bool, PlatformError> {
        let key = format!("leader:{}", self.job_type);
        let result = self.redis.set_nx(&key, "1").expire(self.ttl).await?;
        Ok(result)
    }

    pub async fn release(&self) -> Result<(), PlatformError> {
        let key = format!("leader:{}", self.job_type);
        self.redis.del(&key).await?;
        Ok(())
    }
}
```

**LEADER-001**: Each job type elects exactly one leader. Leader dies → TTL expires → another replica acquires within one TTL cycle.

---

## 5. Feature Flag Management (Part 4 §10.2)

```rust
pub struct FeatureFlag {
    pub flag_key: String,
    pub enabled: bool,
    pub targeting: FlagTargeting,
    pub kill_switch: bool,
}

pub enum FlagTargeting {
    Global(bool),
    Percentage(f64),
    Segment(String),
}
```

Flag changes are domain events (`FeatureFlagChanged`) consumed by all services. Kill-switch flags propagate via Redis pub/sub for sub-second effect. All flag changes follow Maker/Checker pattern.

---

## 6. Structured Log Schema (Part 4 §10.3)

Every service emits:

```json
{
  "timestamp": "2026-07-18T10:15:00.123Z",
  "level": "INFO|WARN|ERROR|DEBUG",
  "service": "orchestration-service",
  "correlation_id": "01HZ...",
  "causation_id": "01HZ...",
  "actor_id": "01HZ...",
  "actor_type": "user|api_key|system",
  "event_type": "PaymentAuthorized",
  "message": "...",
  "metadata": { ... }
}
```

**LOG-SCHEMA-002**: Enforced via shared `platform-logging` crate. No runtime regex — compile-time typed fields.

**LOG-SCHEMA-003**: Sensitive fields (credentials, PAN, tokens) excluded. Log-scrubbing layer strips accidentally included data.

---

## 7. Connection Pool Management (Part 9 §9.1)

```rust
pub struct PoolConfig {
    pub max_connections: u32,      // default: 20 (event-sourced), 10 (supporting)
    pub connection_timeout: Duration, // 5s
    pub idle_timeout: Duration,    // 300s
    pub max_lifetime: Duration,    // 1800s
}

// Sizing formula: pool_size = (cpu_cores * 2) + disk_spindles
// Event-sourced services: 20 connections (higher write throughput)
// Supporting services: 10 connections
```

**POOL-003**: When `cl_waiting` > 0 for >5s → alert. When >10 → service degraded.

---

## 8. Infrastructure Degraded Modes (Part 11 §14.1)

### Redis Down

**REDIS-DEGRADED-001**: Local in-memory rate limiter (2× normal limit). Idempotency bypasses Redis → direct event store. Permission cache falls back to Postgres. Alert: `RedisDegradedMode`.

### NATS Down

**NATS-DEGRADED-001**: Outbox relay continues appending to outbox table (events durably stored). Critical alert raised. Outbox table max size: 1M rows; overflow → archive to MinIO.

### ClickHouse Down

**CLICKHOUSE-DEGRADED-001**: Dashboard endpoints return last-cached results with `X-Data-Stale: true` header. Staleness >1hr → HTTP 503 with `Retry-After: 60`.

### OpenSearch Down

**OPENSEARCH-DEGRADED-001**: AI Assistant degrades to structured-only mode. Direct Postgres lookups work; semantic search returns "retrieval temporarily unavailable."

### MinIO Down

**MINIO-DEGRADED-001**: Settlement file ingestion queues in local staging dir (max 1GB on pod volume). Files moved to MinIO on recovery.

---

## 9. Envelope Encryption (Part 8 §3)

```rust
pub struct KmsClient {
    kek_id: String,
    dek_cache: HashMap<String, Vec<u8>>,
}

impl KmsClient {
    /// Encrypt: generate DEK, encrypt data with DEK, wrap DEK with KEK
    pub async fn encrypt(&self, data: &[u8], context: &EncryptionContext) -> Result<EncryptedData, PlatformError> {
        let dek = Aes256Gcm::generate_key(&mut OsRng);
        let ciphertext = encrypt_data(&dek, data, &context.aad())?;
        let wrapped_dek = self.wrap_dek(&dek).await?;
        Ok(EncryptedData { ciphertext, wrapped_dek, key_version: self.kek_version })
    }

    /// Decrypt: unwrap DEK with KEK, decrypt data with DEK
    pub async fn decrypt(&self, encrypted: &EncryptedData, context: &EncryptionContext) -> Result<Vec<u8>, PlatformError> {
        let dek = self.unwrap_dek(&encrypted.wrapped_dek).await?;
        decrypt_data(&dek, &encrypted.ciphertext, &context.aad())
    }
}

pub struct EncryptionContext {
    pub operator_id: Uuid,
    pub resource_id: Uuid,
}

impl EncryptionContext {
    pub fn aad(&self) -> Vec<u8> {
        // AAD binds ciphertext to specific context (ENC-012)
        format!("{}:{}", self.operator_id, self.resource_id).into_bytes()
    }
}
```

---

## 10. SSRF Prevention (Part 8 §7.4)

```rust
pub fn validate_outbound_url(url: &Url) -> Result<(), SsrfError> {
    // 1. Scheme must be HTTPS
    if url.scheme() != "https" {
        return Err(SsrfError::NonHttpsScheme);
    }

    // 2. Resolve DNS
    let ips = resolve_dns(url.host_str().ok_or(SsrfError::NoHost)?)?;

    // 3. Check resolved IPs against deny-list
    for ip in &ips {
        if is_private_or_reserved(ip) {
            return Err(SsrfError::PrivateIpDetected { ip: *ip });
        }
    }

    // 4. No redirect following without re-validation
    Ok(())
}

fn is_private_or_reserved(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}
```

---

## 11. Audit Log Tamper-Evidence (Part 8 §16.4)

```rust
pub struct AuditEntry {
    pub id: Uuid,
    pub previous_entry_hash: Option<Vec<u8>>,  // SHA-256 of previous entry
    pub entry_hash: Vec<u8>,                    // SHA-256 of this entry + previous_hash
    // ... other fields
}

impl AuditEntry {
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.previous_entry_hash.as_deref().unwrap_or(&[]));
        hasher.update(self.action.as_bytes());
        hasher.update(self.actor_id.as_bytes());
        hasher.update(self.occurred_at.to_rfc3339().as_bytes());
        hasher.finalize().to_vec()
    }
}
```

**AUD-005**: Daily verification job walks chain, alerts on breaks.

---

## 12. Card Testing Abuse Prevention (Part 8 §17.5)

```rust
pub struct AbuseDetector {
    redis: RedisPool,
}

impl AbuseDetector {
    /// Operator-level velocity: max 100 auth attempts per 15min window
    pub async fn check_operator_velocity(&self, operator_id: Uuid) -> Result<(), AbuseError> {
        let key = format!("abuse:auth_velocity:{}", operator_id);
        let count = self.redis.incr(&key).await?;
        self.redis.expire(&key, Duration::from_secs(900)).await?;
        if count > 100 {
            return Err(AbuseError::OperatorVelocityExceeded);
        }
        Ok(())
    }

    /// Zero-amount auth rate limit: max 10 per card BIN per hour
    pub async fn check_zero_auth_rate(&self, bin: &str) -> Result<(), AbuseError> {
        let key = format!("abuse:zero_auth:{}", bin);
        let count = self.redis.incr(&key).await?;
        self.redis.expire(&key, Duration::from_secs(3600)).await?;
        if count > 10 {
            return Err(AbuseError::ZeroAuthRateExceeded);
        }
        Ok(())
    }

    /// Cross-IP detection: same payment method token from >3 IPs in 5min
    pub async fn check_cross_ip(&self, token_id: Uuid, ip: IpAddr) -> Result<(), AbuseError> {
        let key = format!("abuse:cross_ip:{}", token_id);
        self.redis.sadd(&key, ip.to_string()).await?;
        self.redis.expire(&key, Duration::from_secs(300)).await?;
        let count = self.redis.scard(&key).await?;
        if count > 3 {
            return Err(AbuseError::CrossIpDetected);
        }
        Ok(())
    }
}
```

---

## 13. WebAuthn MFA (Part 8 §16.2)

```rust
pub struct WebAuthnEnrollment {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub attestation_object: Vec<u8>,
    pub sign_count: u32,
}

pub struct BackupCode {
    pub code_hash: Vec<u8>, // argon2id hash
    pub used: bool,
    pub used_at: Option<DateTimeWithTimeZone>,
}

// AUTH-011: WebAuthn minimum for Admin/Finance roles
// AUTH-012: Attestation verification on enrollment
// AUTH-013: Session token includes hash of credential_id
// AUTH-014: 10 backup codes generated at enrollment
// AUTH-015: Login notification on new device/IP
// AUTH-016: All sessions invalidated on password change
```

---

## 14. Data Retention Automation (Part 8 §14 / BIZ-051)

```rust
pub struct DataRetentionEnforcer {
    db: DatabaseConnection,
}

impl DataRetentionEnforcer {
    pub async fn enforce(&self, policy: &RetentionPolicy) -> Result<RetentionResult, PlatformError> {
        let terminal_aggregates = self.find_terminal_aggregates(policy).await?;

        for aggregate in terminal_aggregates {
            // Move event stream to archive
            self.archive_events(aggregate.id, &policy.archive_table).await?;

            // Log archival action
            self.log_retention_action(RetentionAuditEntry {
                aggregate_id: aggregate.id,
                events_archived: aggregate.event_count,
                retention_policy: policy.name.clone(),
                archived_at: Utc::now(),
                legal_floor_respected: true,
            }).await?;
        }

        Ok(RetentionResult { archived: terminal_aggregates.len() })
    }
}
```

---

## 15. Scheduled Jobs Registry

| Job | Service | Schedule | Leader Election |
|---|---|---|---|
| JOB-001: Subscription renewal | subscription-service | Per billing cycle | Yes |
| JOB-002: Dunning retry | subscription-service | Configurable | Yes |
| JOB-003: Settlement polling | reconciliation-service | Per connector config | Yes |
| JOB-004: Invoice overdue | invoice-service | Daily | Yes |
| JOB-005: AI re-embedding | ai-assistant-service | Hourly | Yes |
| JOB-006: Exception aging alerts | reconciliation-service | Daily | Yes |
| JOB-007: Auth expiry sweep | orchestration-service | Every 5 min | Yes |
| JOB-008: Stuck Authorizing | orchestration-service | Every 1 min | Yes |
| JOB-009: Data retention | per-service | Daily | Yes |
| JOB-010: Outbox relay health | per-service | Every 30s | No (all replicas) |
| JOB-011: Renewal idempotency | subscription-service | Per renewal | Yes |
| JOB-012: Stuck Capturing/Refunding | orchestration-service | Every 1 min | Yes |
| LEDGER-VERIFY-001: Ledger balance | reconciliation-service | Daily | Yes |
| CONSIST-001: Cross-service consistency | reconciliation-service | Daily | Yes |
| AUD-005: Audit hash chain verify | iam-service | Daily | Yes |
| SETTLE-AGE-001: Settlement age check | reconciliation-service | Daily | Yes |
| FEE-VAR-001: Fee variance report | reconciliation-service | Daily | Yes |
| SETTLE-ADJUST-001: Settlement adjustment | reconciliation-service | Per event | Yes |
| WEBHOOK-RETRY-001: Webhook retry | webhook-delivery-service | Every 30s | Yes |
| WEBHOOK-CLEANUP-001: Webhook cleanup | webhook-delivery-service | Daily | Yes |
| TOKEN-EXPIRY-001: Token expiry check | orchestration-service | Daily | Yes |
| RISK-STATS-001: Risk stats aggregation | risk-service | Hourly | Yes |
| CHARGEBACK-DEADLINE-001: Representment deadline | dispute-service | Daily | Yes |

---

## 16. API Key Lifecycle Automation (Part 8 §16.14)

```rust
pub struct ApiKeyLifecycleManager {
    notification_service: NotificationClient,
}

impl ApiKeyLifecycleManager {
    pub async fn check_and_notify(&self) -> Result<(), PlatformError> {
        let expiring_keys = self.find_keys_expiring_within(30).await?;
        for key in expiring_keys {
            self.notification_service.send(ApiKeyExpiringNotification {
                principal_id: key.principal_id,
                key_name: key.name,
                days_until_expiry: key.days_until_expiry(),
            }).await?;
        }

        let expired_keys = self.find_expired_keys().await?;
        for key in expired_keys {
            self.revoke_key(key.id, "auto-expired").await?;
        }

        Ok(())
    }
}
```

---

## 17. CORS Policy (Part 8 §12.3)

```rust
pub fn cors_middleware(allowed_origins: &[String]) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowedOrigins::list(allowed_origins)) // exact match only
        .allow_methods([POST, PATCH, DELETE, OPTIONS])        // Only POST — no GET/PUT/PATCH/DELETE (strict protobuf)
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, X_API_KEY, X_IDEMPOTENCY_KEY, X_REQUEST_ID, X_CSRF_TOKEN])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600)) // 1 hour (not 24h for auth endpoints)
}
```

**PROTO-CORS-001**: Only `POST` and `OPTIONS` methods are allowed. Any GET/PUT/PATCH/DELETE request returns `405 Method Not Allowed`.

---

## 18. Request Size & Timeout Limits (Part 10 §6)

```rust
pub struct RequestLimits {
    pub max_standard_body: usize,      // 1MB
    pub max_document_upload: usize,    // 10MB
    pub max_payment_creation: usize,   // 100KB
    pub general_timeout: Duration,     // 30s
    pub checkout_timeout: Duration,    // 10s
    pub acquirer_authorize_timeout: Duration, // 15s
    pub acquirer_settlement_timeout: Duration, // 30s
}
```

---

## 19. Input Validation (Part 10 §6.2) — Strict Protobuf

All API inputs are protobuf messages. Validation happens at two levels:

### Level 1: Protobuf Schema Validation (API Gateway)

```rust
pub fn validate_protobuf_request<T: prost::Message>(bytes: &[u8]) -> Result<T, ValidationError> {
    // 1. Decode protobuf binary
    let message = T::decode(bytes)
        .map_err(|e| ValidationError::InvalidProtobuf { detail: e.to_string() })?;

    // 2. Required field validation (proto3 optional fields check)
    // 3. Enum value validation (unknown enum values rejected)
    // 4. Oneof field validation

    Ok(message)
}
```

### Level 2: Domain Validation (Command Handler)

```rust
// Semantic validation in command handler (not at gateway):
// - UUIDv7 format validation
// - ISO 4217 currency code validation
// - Amount range validation (non-negative, within currency precision)
// - Business rule validation (e.g., "amount must be positive")
```

**PROTO-VALIDATE-001**: Malformed protobuf requests are rejected at the API Gateway with HTTP 400 before reaching any backend service.

**PROTO-VALIDATE-002**: Valid protobuf with invalid business data is rejected at the command handler with an `ErrorDetail` response.

---

## 20. Request Correlation (Part 10 §12.1 REQ-003)

```rust
// Every inbound request generates UUIDv7 request_id
// Propagated in:
//   - gRPC metadata: x-request-id
//   - HTTP header: X-Request-ID
//   - Log context (LOG-SCHEMA-001)
//   - Distributed trace (OBS-005)
//   - Error responses (API-005)
```

---

## 21. CSRF Protection (Part 8 §16.7)

```rust
pub struct CsrfGuard {
    session_store: RedisPool,
}

impl CsrfGuard {
    pub async fn generate_token(&self, session_id: &str) -> Result<String, PlatformError> {
        let token = generate_random_bytes(32); // 256-bit
        self.session_store.set(
            &format!("csrf:{}", session_id),
            &token,
            Duration::from_secs(1800),
        ).await?;
        Ok(base64::encode(&token))
    }

    pub async fn validate(&self, session_id: &str, token: &str) -> Result<(), PlatformError> {
        let stored = self.session_store.get(&format!("csrf:{}", session_id)).await?;
        if stored != base64::decode(token)? {
            return Err(PlatformError::AuthorizationDenied("CSRF token mismatch".into()));
        }
        Ok(())
    }
}
```

---

## 22. Database Connection Security (Part 9 §12.6)

```toml
# All Postgres connections use:
# sslmode=verify-full (not just require)
# Connection string validation in CI: no sslmode=disable or sslmode=allow

[database]
url = "postgres://user:password@host:5432/db?sslmode=verify-full"
```

---

## 23. Event Store Integrity Verification (Part 5 §12.5)

```rust
pub struct EventStoreIntegrityChecker {
    db: DatabaseConnection,
}

impl EventStoreIntegrityChecker {
    pub async fn verify_aggregate(&self, aggregate_id: Uuid) -> Result<IntegrityResult, PlatformError> {
        let events = self.load_events(aggregate_id).await?;

        // Check: no sequence gaps
        for window in events.windows(2) {
            if window[1].event_sequence - window[0].event_sequence != 1 {
                return Ok(IntegrityResult::GapDetected {
                    aggregate_id,
                    gap_between: (window[0].event_sequence, window[1].event_sequence),
                });
            }
        }

        // Check: no duplicate sequences
        let sequences: Vec<i64> = events.iter().map(|e| e.event_sequence).collect();
        if sequences.iter().collect::<HashSet<_>>().len() != sequences.len() {
            return Ok(IntegrityResult::DuplicateDetected { aggregate_id });
        }

        // Check: first event is a root-creation event
        if !events[0].event_type.ends_with("Created") {
            return Ok(IntegrityResult::MissingRootEvent { aggregate_id });
        }

        Ok(IntegrityResult::Valid)
    }
}
```

---

## 24. Concurrent Request Rate Limiting (Part 4 §10.6)

```rust
pub async fn check_concurrent_limit(
    redis: &RedisPool,
    api_key_id: &str,
    max_concurrent: u32,
) -> Result<(), PlatformError> {
    let key = format!("concurrent:{}", api_key_id);
    let count = redis.incr(&key).await?;
    if count > max_concurrent as i64 {
        redis.decr(&key).await?; // rollback
        return Err(PlatformError::RateLimited { retry_after_ms: 1000 });
    }
    // TTL safety net: auto-decrement after 30s
    redis.expire_at(&key, Utc::now() + Duration::from_secs(30)).await?;
    Ok(())
}
```

---

## 25. SFTP Settlement File Security (Part 7 §9.1)

```rust
pub struct SftpClient {
    host_key_fingerprints: Vec<String>, // pinned
}

impl SftpClient {
    pub async fn connect(&self, config: &SftpConfig) -> Result<SftpSession, PlatformError> {
        // 1. Connect with TLS
        let session = SshSession::connect(&config.host, config.port).await?;

        // 2. Verify host key against pinned fingerprints
        let server_fingerprint = session.host_key_fingerprint();
        if !self.host_key_fingerprints.contains(&server_fingerprint) {
            return Err(PlatformError::AuthorizationDenied("SSH host key mismatch".into()));
        }

        // 3. Authenticate with encrypted credentials
        session.authenticate(&config.credentials).await?;

        Ok(session)
    }

    pub async fn download_settlement_file(&self, path: &str) -> Result<SettlementFile, PlatformError> {
        let content = self.session.read_file(path).await?;
        let checksum = Sha256::digest(&content);

        // Log to audit trail
        self.audit_log(SettlementFileDownloaded {
            path: path.to_string(),
            checksum: hex::encode(checksum),
            downloaded_at: Utc::now(),
        }).await?;

        Ok(SettlementFile { content, checksum: hex::encode(checksum) })
    }
}
```

---

## 26. Cursor Pagination Security (Part 10 §10.1)

```rust
pub struct SecureCursor {
    encryption_key: [u8; 32], // AES-256 key
}

impl SecureCursor {
    pub fn encode(&self, cursor: &CursorData) -> Result<String, PlatformError> {
        let json = serde_json::to_vec(cursor)?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let cipher = Aes256Gcm::new(&self.encryption_key.into());
        let ciphertext = cipher.encrypt(&nonce, json.as_ref())?;
        Ok(base64::encode([&nonce[..], &ciphertext].concat()))
    }

    pub fn decode(&self, encoded: &str, expected_filters: &str) -> Result<CursorData, PlatformError> {
        let bytes = base64::decode(encoded)?;
        let (nonce, ciphertext) = bytes.split_at(12);
        let cipher = Aes256Gcm::new(&self.encryption_key.into());
        let json = cipher.decrypt(nonce.into(), ciphertext)?;
        let cursor: CursorData = serde_json::from_slice(&json)?;

        // Validate filter_hash matches current request filters
        if cursor.filter_hash != hash_filters(expected_filters) {
            return Err(PlatformError::Validation(ValidationError::InvalidIdempotencyKey));
        }

        // Validate expiry (1 hour)
        if cursor.expires_at < Utc::now() {
            return Err(PlatformError::Validation(ValidationError::InvalidIdempotencyKey));
        }

        Ok(cursor)
    }
}
```

---

## 27. Webhook Payload Schema Versioning (Part 10 §10.2)

```rust
pub struct WebhookPayload {
    pub schema_version: String, // "2026-07-18"
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub data: serde_json::Value,
}
```

New fields are additive within a schema version. Breaking changes increment version. Merchants register preferred schema version per endpoint.

---

## 28. API Staleness Disclosure (Part 10 §10.4)

```rust
pub struct StalenessConfig {
    pub max_lag_seconds: u64, // per read-model, e.g., 300 for reconciliation
}

pub fn check_staleness(
    as_of: DateTime<Utc>,
    max_lag: Duration,
) -> Result<(), StalenessError> {
    let lag = Utc::now().signed_duration_since(as_of);
    if lag > max_lag {
        return Err(StalenessError::DataTooStale {
            lag_seconds: lag.num_seconds(),
            max_allowed: max_lag.num_seconds(),
        });
    }
    Ok(())
}
```

---

## 29. Webhook Delivery Backpressure (Part 10 §10.3)

```rust
pub struct WebhookThrottler {
    redis: RedisPool,
}

impl WebhookThrottler {
    pub async fn should_throttle(&self, endpoint_id: Uuid) -> Result<bool, PlatformError> {
        // Per-endpoint concurrent limit: max 5 in-flight
        let key = format!("webhook:inflight:{}", endpoint_id);
        let count = self.redis.incr(&key).await?;
        self.redis.expire(&key, Duration::from_secs(10)).await?;
        Ok(count > 5)
    }

    pub async fn adaptive_throttle(&self, endpoint_id: Uuid) -> Result<bool, PlatformError> {
        // If >50% failure rate over last 100 deliveries → throttle to 1/30s
        let failure_rate = self.get_failure_rate(endpoint_id, 100).await?;
        if failure_rate > 0.5 {
            let key = format!("webhook:throttle:{}", endpoint_id);
            self.redis.set(&key, "1", Duration::from_secs(30)).await?;
            return Ok(true);
        }
        Ok(false)
    }
}
```

---

## 35. Gap: Outbound Webhook Delivery Service (Critical)

Merchants integrate with the platform via outbound webhooks. This is the primary programmatic integration path.

### Architecture

```
Domain Event (NATS) → Webhook Delivery Service → Merchant Endpoint
                      (async, retry, signing)
```

### Webhook Subscription Management

```rust
// WebhookSubscription aggregate (owned by api-gateway or dedicated service)
pub struct WebhookSubscription {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub url: String,                    // HTTPS only, SSRF-validated
    pub event_types: Vec<WebhookEventType>,
    pub secret_hash: Vec<u8>,           // argon2id hash of signing secret
    pub status: SubscriptionStatus,
    pub created_at: DateTimeWithTimeZone,
}

pub enum WebhookEventType {
    PaymentCreated,
    PaymentAuthorized,
    PaymentCaptured,
    PaymentFailed,
    PaymentRefunded,
    PaymentVoided,
    SettlementMatched,
    SettlementUnmatched,
    ChargebackReceived,
    ChargebackResolved,
}
```

### Webhook Delivery Entity

```rust
pub struct WebhookDelivery {
    pub delivery_id: Uuid,
    pub subscription_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: String,                // JSON payload
    pub signature: String,              // HMAC-SHA256(secret, payload)
    pub status: DeliveryStatus,
    pub attempt_count: u32,
    pub last_attempt_at: Option<DateTimeWithTimeZone>,
    pub next_retry_at: Option<DateTimeWithTimeZone>,
    pub response_status_code: Option<u16>,
    pub response_body: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    PermanentlyFailed,  // after max retries exhausted
}
```

### Delivery Flow

```
1. Domain event published to NATS (e.g., PaymentAuthorized)
2. Webhook Delivery Service subscribes to all payment events
3. For each event:
   a. Find all active WebhookSubscriptions for the operator that include this event_type
   b. For each subscription:
      i.   Create WebhookDelivery record (status=Pending)
      ii.  Build JSON payload (event_type, data, timestamp, subscription_id)
      iii. Sign payload: HMAC-SHA256(subscription.secret, payload)
      iv.  POST to merchant URL with signature in X-Webhook-Signature header
      v.   If HTTP 2xx: status=Delivered
      vi.  If non-2xx or timeout: increment attempt_count, schedule retry
```

### Retry Policy

```
Attempt 1: immediate
Attempt 2: 1 second delay
Attempt 3: 5 seconds delay
Attempt 4: 30 seconds delay
Attempt 5: 5 minutes delay
Attempt 6: 30 minutes delay
Attempt 7: 2 hours delay
Attempt 8: 8 hours delay (final attempt)
After 8 failures: status=PermanentlyFailed, alert merchant
```

### Payload Signing (Merchant Verification)

```rust
pub fn sign_webhook_payload(secret: &[u8], payload: &[u8]) -> String {
    let mut hmac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    hmac.update(payload);
    format!("sha256={}", hex::encode(hmac.finalize().into_bytes()))
}

pub fn verify_webhook_signature(secret: &[u8], payload: &[u8], signature: &str) -> bool {
    let expected = sign_webhook_payload(secret, payload);
    // Constant-time comparison to prevent timing attacks
    expected.len() == signature.len()
        && expected.bytes().zip(signature.bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}
```

### Webhook Security

- **WEBHOOK-SEC-001**: URLs must be HTTPS (SSRF-validated)
- **WEBHOOK-SEC-002**: Payload signed with merchant-specific HMAC-SHA256
- **WEBHOOK-SEC-003**: Signature in `X-Webhook-Signature` header
- **WEBHOOK-SEC-004**: Delivery log retained for 30 days (audit trail)
- **WEBHOOK-SEC-005**: Merchant can view delivery history on dashboard
- **WEBHOOK-SEC-006**: Rate limit: max 1000 deliveries per minute per subscription

### Background Job

| Job | Service | Schedule | Description |
|---|---|---|---|
| WEBHOOK-RETRY-001 | webhook-delivery-service | Every 30s | Process pending retries, deliver webhooks |
| WEBHOOK-CLEANUP-001 | webhook-delivery-service | Daily | Archive deliveries older than 30 days |

---

## 30. Complete Rate Limiting Table (Part 10 §11)

All endpoints use RESTful URL paths with protobuf-encoded bodies.

| Service.Method | Limit | Window | Per |
|---|---|---|---|
| `POST /v1/payment-intents` | 1000 | 60s | ApiKey |
| `OrchestrationService/AuthorizePaymentIntent` | 1000 | 60s | ApiKey |
| `POST /v1/payment-intents/:id/capture` | 500 | 60s | ApiKey |
| `OrchestrationService/VoidPaymentIntent` | 500 | 60s | ApiKey |
| `POST /v1/payment-intents/:id/refund` | 200 | 60s | ApiKey |
| `OrchestrationService/GetPaymentIntent` | 500 | 60s | ApiKey |
| `POST /v1/payment-intents/search` | 100 | 60s | ApiKey |
| `POST /v1/invoices` | 50 | 60s | ApiKey |
| `POST /v1/invoices/search` | 100 | 60s | ApiKey |
| `SubscriptionService/CreateSubscription` | 50 | 60s | ApiKey |
| `POST /v1/subscriptions/search` | 100 | 60s | ApiKey |
| `RoutingPolicyService/ActivateRoutingPolicy` | 10 | 60s | ApiKey |
| `IAMService/Authenticate` | 10 | 60s | IP |
| `AIAssistantService/AskQuestion` | 30 | 60s | ApiKey |
| `DocumentService/UploadDocument` | 20 | 60s | ApiKey |
| `AnalyticsService/GetDashboard` | 100 | 60s | ApiKey |
| `ComplianceService/SubmitKybEvidence` | 10 | 60s | ApiKey |
| `WebhookService/CreateWebhookSubscription` | 10 | 60s | ApiKey |
| `WebhookService/ListWebhookDeliveries` | 50 | 60s | ApiKey |

---

## 31. Load Testing Specs (Part 11 §8.1)

### Checkout Hot Path

```rust
pub struct LoadTestScenario {
    pub name: String,
    pub target_tps: u32,
    pub duration_seconds: u32,
    pub ramp_up_seconds: u32,
    pub assertions: Vec<LoadTestAssertion>,
}

pub struct LoadTestAssertion {
    pub metric: String,        // "p99_latency_ms" | "error_rate" | "throughput"
    pub threshold: f64,
    pub operator: ComparisonOp, // LessThan | GreaterThan
}

// Scenario 1: Single-hop authorization
LoadTestScenario {
    name: "single_hop_authorize".into(),
    target_tps: 500,
    duration_seconds: 300,
    ramp_up_seconds: 30,
    assertions: vec![
        LoadTestAssertion { metric: "p99_latency_ms".into(), threshold: 2000.0, operator: ComparisonOp::LessThan },
        LoadTestAssertion { metric: "error_rate".into(), threshold: 0.01, operator: ComparisonOp::LessThan },
    ],
}

// Scenario 2: Failover under load
LoadTestScenario {
    name: "failover_under_load".into(),
    target_tps: 200,
    duration_seconds: 300,
    ramp_up_seconds: 30,
    assertions: vec![
        LoadTestAssertion { metric: "p99_latency_ms".into(), threshold: 5000.0, operator: ComparisonOp::LessThan },
        LoadTestAssertion { metric: "failover_success_rate".into(), threshold: 0.95, operator: ComparisonOp::GreaterThan },
    ],
}

// Scenario 3: Concurrent payments same card
LoadTestScenario {
    name: "concurrent_same_card".into(),
    target_tps: 100,
    duration_seconds: 60,
    assertions: vec![
        LoadTestAssertion { metric: "double_authorization_count".into(), threshold: 0.0, operator: ComparisonOp::LessThan },
    ],
}

// Scenario 4: Settlement ingestion burst
LoadTestScenario {
    name: "settlement_burst".into(),
    target_tps: 50,
    duration_seconds: 600,
    assertions: vec![
        LoadTestAssertion { metric: "settlement_match_rate".into(), threshold: 0.99, operator: ComparisonOp::GreaterThan },
    ],
}
```

### Performance Regression Detection

```rust
pub struct PerformanceBaseline {
    pub metric_name: String,
    pub baseline_value: f64,
    pub regression_threshold_percent: f64, // default: 15%
}

// If current p99 > baseline * (1 + threshold) → build fails
pub fn check_regression(current: f64, baseline: &PerformanceBaseline) -> bool {
    current <= baseline.baseline_value * (1.0 + baseline.regression_threshold_percent / 100.0)
}
```

---

## 32. Chaos Testing Specs (Part 11 §8.2 + §14.3)

### Infrastructure Chaos

| Scenario | Inject | Validate | Alert |
|---|---|---|---|
| Postgres failover | Kill primary Postgres | Read-path serves from replica; write-path fails fast | Write errors < 5s |
| Redis eviction | Fill Redis memory | Cache misses fall through to Postgres (REDIS-001) | No correctness loss |
| NATS partition | Block NATS connectivity | Outbox relay retries; no events lost | Relay lag monitored |
| MinIO unavailability | Block MinIO access | Document uploads queue locally | Queue size < 1GB |
| OpenSearch down | Stop OpenSearch | AI Assistant degrades to structured-only mode | Graceful degradation |
| GPU pool saturation | Overload Ollama | AI Gateway circuit-breaks (AIGW-005) | Graceful degradation |

### Payment-Flow Chaos

| Scenario | Inject | Validate |
|---|---|---|
| Event loss | Kill outbox relay mid-transaction | Events eventually published (outbox retains) |
| Response loss | Mock acquirer returns success but drops response | Status-check detects actual state |
| Concurrent mutation | Send simultaneous capture + void | Optimistic concurrency rejects one |
| Double authorization | Send concurrent authorize for same intent | Only one succeeds |
| Stuck authorizing | Mock acquirer never responds | JOB-008 detects and status-checks |

---

## 33. Data Masking Service (Part 9 §12.5)

```rust
pub struct DataMaskingService {
    minio_client: MinIOClient,
    db_client: DatabaseConnection,
}

impl DataMaskingService {
    pub async fn mask_for_staging(&self, snapshot: &DatabaseSnapshot) -> Result<MaskedSnapshot, PlatformError> {
        let mut masked = snapshot.clone();

        // 1. Replace real acquirer credentials with sandbox equivalents
        for link in &mut masked.merchant_acquirer_links {
            link.encrypted_config = self.generate_sandbox_credentials(&link.connector_id).await?;
        }

        // 2. Replace real KYB document references with synthetic ones
        for doc in &mut masked.kyb_documents {
            doc.minio_key = format!("synthetic/kyb/{}.pdf", doc.id);
        }

        // 3. Replace real card tokens with synthetic tokens
        for token in &mut masked.payment_method_tokens {
            token.acquirer_token_reference = format!("tok_synthetic_{}", token.id);
        }

        // 4. Replace real email addresses with synthetic addresses
        for principal in &mut masked.principals {
            if let Some(ref email) = principal.email {
                principal.email = Some(format!("user_{}@synthetic.test", principal.id));
            }
        }

        Ok(masked)
    }
}
```

**MASK-002**: Staging/test environments use separate MinIO buckets and database instances — never shared with production.

**MASK-003**: CI gate scans database fixtures for PII patterns (email regex, card number patterns, API key patterns) and fails the build if real data is detected.

---

## 34. Runbook Implementations (Part 11 §14.13)

### Runbook: Database Complete Cluster Loss

```
Detection: PostgreSQL primary + all replicas unreachable
Severity: SEV-1 (Critical)
RTO: < 5 minutes

Steps:
1. Verify: kubectl get pods -n production | grep postgres → all CrashLoopBackOff
2. Alert: PagerDuty SEV-1 → on-call SRE + engineering lead
3. Restore: Restore from WAL archival + base backup to new primary
4. Validate: Run consistency check (LEDGER-VERIFY-001, CONSIST-001)
5. Replay: Replay outbox entries between backup and failure time
6. Rebuild: Trigger projection rebuild for affected services
7. Verify: Health checks pass for all dependent services
8. Communicate: Status page update → "Database recovered, all services operational"
9. Post-incident: Create incident report within 48 hours
```

### Runbook: NATS Complete Cluster Loss

```
Detection: NATS cluster unreachable, outbox relay lag increasing
Severity: SEV-1 (Critical)
RTO: < 10 minutes

Steps:
1. Verify: nats-cli server list → all nodes unreachable
2. Alert: PagerDuty SEV-1
3. Provision: Deploy new NATS cluster from infrastructure-as-code
4. Recreate: Apply stream/consumer configuration from checked-in config files
5. Replay: Run outbox relay against new cluster (outbox table retains unpublished events)
6. Validate: Consumer lag returns to zero; all projections current
7. Verify: Event-driven workflows (notifications, analytics) resume
8. Communicate: Status page update
9. Post-incident: Root cause analysis
```

### Runbook: AI Model Degradation

```
Detection: ai_answer_accuracy < 80% of baseline for 1 hour
Severity: SEV-2 (High)
RTO: < 30 minutes

Steps:
1. Check: Ollama inference pool health (GPU utilization, memory)
2. Check: Recent model version changes (MODEL-PIN-002)
3. Check: Retrieval quality metrics (citation_hit_rate)
4. If model changed: rollback to previous version (MODEL-ROLLBACK-001)
5. If retrieval degraded: trigger re-embedding (JOB-005)
6. If Ollama issue: restart inference pool
7. If persistent: degrade to raw-data mode (AIGW-005)
8. Validate: Quality metrics return to baseline within 1 hour
9. Communicate: Notify affected operators
```

### Runbook: Secret Compromise

```
Detection: Unusual credential access patterns, SIEM alert
Severity: SEV-1 (Critical)
RTO: < 15 minutes

Steps:
1. Isolate: Revoke compromised credentials immediately (no Maker/Checker needed)
2. Rotate: Rotate KEK (emergency rotation per KMP-004)
3. Re-encrypt: Run KEKReEncryptionJob to re-wrap all DEKs
4. Audit: Review credential access logs for scope of compromise
5. Notify: Alert affected operators and compliance team
6. Forensic: Preserve logs for investigation
7. Post-incident: Full review within 48 hours; update access controls
```

### Runbook: Acquirer Outage

```
Detection: Circuit breaker open for > 5 minutes
Severity: SEV-2 (High)
RTO: < 5 minutes (automatic failover)

Steps:
1. Verify: Check acquirer status page (if available)
2. Verify: Circuit breaker state in Redis
3. Verify: Failover routing is working (PaymentAuthorizationAttempted events)
4. If single acquirer: confirm automatic failover (no action needed)
5. If multiple acquirers: activate emergency maintenance mode (feature flag)
6. Communicate: Status page → "Payment processing via failover"
7. Monitor: Authorization rate should recover with secondary acquirers
8. Post-incident: Contact acquirer support; document outage timeline
```
# APPLIED SIMPLIFICATIONS (from Gap Analysis)

The following items from the original specification have been simplified per the comprehensive gap analysis:
- Hash-linked audit chain: REPLACED with append-only audit tables + WAL archival
- Concurrent request counting: REPLACED with standard Redis sliding window rate limiting
- WebAuthn MFA as only option: REPLACED with TOTP (primary) + WebAuthn (Phase 1 upgrade)
- Event Store Integrity Checker: Simplified to periodic sequence gap check
- Separate MinIO: REPLACED with direct S3-compatible API calls
- ClickHouse at launch: Deferred to Phase 2, PostgreSQL analytics initially
- Event Schema Registry as Git repo: Simplified to shared protobuf workspace crate
See docs/analysis/001-comprehensive-gap-design-overengineering-analysis.md §4 for full rationale.
