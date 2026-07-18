# 04 — connector-gateway (BC-04 Gateway Connector Framework)

Anti-Corruption Layer. Translates N acquirer APIs into one normalized internal protocol.

---

## 1. Core Abstraction: AcquirerConnector Trait

```rust
#[async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> ConnectorId;
    fn capabilities(&self) -> ConnectorCapabilities;

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError>;
    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError>;
    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError>;
    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError>;
    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError>;

    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError>;
    fn verify_webhook_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<(), ConnectorError>;
    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError>;

    fn onboarding_schema(&self) -> OnboardingSchema;
    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<(), ConnectorError>;
}
```

## 2. Capability Flags

```rust
pub struct ConnectorCapabilities {
    pub supports_partial_capture: bool,
    pub supports_partial_refund: bool,
    pub supports_native_idempotency_key: bool,
    pub supports_webhook_settlement: bool,
    pub supports_realtime_status_check: bool,
    pub supported_card_schemes: Vec<CardScheme>,
    pub supported_currencies: Vec<CurrencyCode>,
    pub settlement_format: SettlementFormat,
}
```

## 3. Normalized Types

```rust
pub struct AuthorizeRequest {
    pub payment_method_token: String,
    pub amount: Money,
    pub idempotency_key: String,
    pub card_scheme: CardScheme,
    pub metadata: Option<serde_json::Value>,
}

pub struct AuthorizeResponse {
    pub status: AuthorizeStatus, // Approved | Declined | Requires3DS | PartialApproval
    pub acquirer_reference: Option<String>,
    pub decline_reason: Option<DeclineReason>,
    pub approved_amount: Option<Money>, // for partial auth
    pub three_ds_data: Option<ThreeDsData>, // for 3DS step-up
    pub latency_ms: u32,
}

pub enum AuthorizeStatus {
    Approved,
    Declined,
    Requires3DS,
    PartialApproval,
}
```

## 4. Circuit Breaker

```rust
pub struct CircuitBreaker {
    state: CircuitState, // Closed | Open | HalfOpen
    error_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    open_duration: Duration, // default: 60s
    error_threshold: f64,    // default: 0.5 (50%)
    window: Duration,        // default: 30s
}
```

## 5. Per-Connector Retry Config

```rust
pub struct ConnectorRetryConfig {
    pub max_retries: u8,
    pub initial_backoff_ms: u32,
    pub backoff_multiplier: f32,
    pub max_backoff_ms: u32,
    pub jitter_percent: f32,
    pub retryable_error_codes: Vec<ConnectorError>,
}
```

## 6. Onboarding Schema & Credential Handling

### 6.1 Dynamic Onboarding Schema

Each connector declares its configuration fields. The dashboard renders forms dynamically.

```rust
pub struct OnboardingSchema {
    pub connector_id: String,
    pub fields: Vec<OnboardingField>,
}

pub struct OnboardingField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub label: String,
    pub validation_regex: Option<String>,
    pub help_text: Option<String>,
}

pub enum FieldType {
    String,
    Password,      // masked input
    Url,
    Integer,
    Select { options: Vec<SelectOption> },
}
```

**Example — Checkout.com Connector:**

```rust
impl AcquirerConnector for CheckoutComConnector {
    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "checkout_com".into(),
            fields: vec![
                OnboardingField {
                    name: "api_key".into(),
                    field_type: FieldType::Password,
                    required: true,
                    label: "Secret Key".into(),
                    validation_regex: Some(r"^sk_(test|live)_[a-zA-Z0-9]+$".into()),
                    help_text: Some("Find in Dashboard > Settings > API Keys".into()),
                },
                OnboardingField {
                    name: "environment".into(),
                    field_type: FieldType::Select {
                        options: vec![
                            SelectOption { value: "sandbox".into(), label: "Sandbox (Test)".into() },
                            SelectOption { value: "production".into(), label: "Production (Live)".into() },
                        ],
                    },
                    required: true,
                    label: "Environment".into(),
                    validation_regex: None,
                    help_text: None,
                },
            ],
        }
    }
}
```

### 6.2 Credential Security

```rust
pub struct EncryptedConnectorConfig {
    pub encrypted_config: Vec<u8>, // envelope-encrypted JSON blob
    pub dek_wrapped: Vec<u8>,     // DEK wrapped by platform KEK
}

// CRED-001: All credentials encrypted at rest via envelope encryption
// CRED-002: Credentials never returned in plaintext via read API
// CRED-003: validate_credentials uses sandbox/status-check, never live-money call
```

**Credential Lifecycle:**

```rust
pub enum CredentialStatus {
    Active,
    Rotating,      // new key validated, old key still valid
    Expired,
    Revoked,
}

pub struct ConnectorCredential {
    pub link_id: Uuid,
    pub connector_id: String,
    pub encrypted_config: EncryptedConnectorConfig,
    pub status: CredentialStatus,
    pub previous_config: Option<EncryptedConnectorConfig>, // for dual-key rotation
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub rotated_at: Option<DateTime<Utc>>,
}
```

### 6.3 Credential Validation

```rust
pub async fn validate_and_connect(
    connector: &dyn AcquirerConnector,
    config: &ConnectorConfig,
) -> Result<MerchantAcquirerLink, PlatformError> {
    // 1. Validate credentials against acquirer's sandbox
    connector.validate_credentials(config).await?;

    // 2. Encrypt and store
    let encrypted = kms_client.encrypt(serde_json::to_vec(config)?, &encryption_context).await?;

    // 3. Create link in 'connected_untested' status
    let link = MerchantAcquirerLink::new(connector.connector_id(), encrypted);

    Ok(link)
}
```

---

## 7. Connector Registry

```rust
pub struct ConnectorRegistry {
    connectors: HashMap<String, Box<dyn AcquirerConnector>>,
}

impl ConnectorRegistry {
    pub fn register(&mut self, connector: Box<dyn AcquirerConnector>) {
        self.connectors.insert(connector.connector_id(), connector);
    }

    pub fn get(&self, connector_id: &str) -> Result<&dyn AcquirerConnector, PlatformError> {
        self.connectors.get(connector_id)
            .map(|c| c.as_ref())
            .ok_or_else(|| PlatformError::NotFound {
                resource: "connector".into(),
                id: Uuid::nil(),
            })
    }

    pub fn list_active(&self) -> Vec<&dyn AcquirerConnector> {
        self.connectors.values().map(|c| c.as_ref()).collect()
    }
}
```

---

## 8. Decline Code Normalization

Each connector maintains its own mapping table:

```rust
pub struct DeclineMappingTable {
    mappings: HashMap<String, DeclineReason>,
}

impl DeclineMappingTable {
    pub fn normalize(&self, raw_code: &str) -> DeclineReason {
        self.mappings.get(raw_code)
            .cloned()
            .unwrap_or_else(|| {
                // DECL-001: Unknown → log for improvement, default to UnknownError
                tracing::warn!(raw_code = %raw_code, "Unmapped decline code");
                DeclineReason::UnknownError(raw_code.to_string())
            })
    }
}
```

**Checkout.com Example Mapping:**

| Raw Code | Normalized | Retryable |
|---|---|---|
| ` insufficient_funds` | `InsufficientFunds` | Yes |
| `do_not_honor` | `DoNotHonor` | Configurable |
| `invalid_card_number` | `InvalidCard` | No |
| `expired_card` | `ExpiredCard` | No |
| `card_declined` | `SuspectedFraud` | No |
| `gateway_timeout` | `IssuerUnavailable` | Yes |
| `3ds_failed` | `ThreeDSecureFailed` | Configurable |
| `rate_limit_exceeded` | `RateLimitedByAcquirer` | Yes |

**Telr Example Mapping:**

| Raw Code | Normalized | Retryable |
|---|---|---|
| `110` (Insufficient funds) | `InsufficientFunds` | Yes |
| `103` (Invalid card) | `InvalidCard` | No |
| `104` (Expired card) | `ExpiredCard` | No |
| `109` (Do not honor) | `DoNotHonor` | Configurable |
| `115` (Issuer unavailable) | `IssuerUnavailable` | Yes |

---

## 9. Settlement Format Handling

### 9.1 Webhook Settlement

```rust
pub struct WebhookSettlementHandler {
    connector_id: String,
    signature_verifier: Box<dyn SignatureVerifier>,
}

impl WebhookSettlementHandler {
    pub async fn handle_inbound(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // 1. Verify signature
        self.signature_verifier.verify(headers, body)?;

        // 2. Parse webhook payload
        let event = self.parse_webhook(body)?;

        // 3. Normalize to RawSettlementRecord
        let records = event.settlement_records.into_iter()
            .map(|r| self.normalize_settlement(r))
            .collect();

        Ok(records)
    }
}
```

### 9.2 Polling API Settlement

```rust
pub struct PollingSettlementHandler {
    connector: Box<dyn AcquirerConnector>,
    last_poll_cursor: Option<String>,
}

impl PollingSettlementHandler {
    pub async fn poll(&self) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        let records = self.connector.poll_settlement(PollSettlementRequest {
            since: self.last_poll_cursor.clone(),
            limit: 1000,
        }).await?;

        Ok(records)
    }
}
```

### 9.3 SFTP Settlement

```rust
pub struct SftpSettlementHandler {
    sftp_client: SftpClient,
    connector_id: String,
}

impl SftpSettlementHandler {
    pub async fn watch_and_ingest(&self) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // 1. Connect with host key pinning
        let session = self.sftp_client.connect(&self.sftp_config).await?;

        // 2. List new files (since last successful download)
        let files = session.list_dir("/settlement/").await?;

        let mut all_records = vec![];
        for file in files {
            // 3. Download and verify checksum
            let content = session.download(&file.path).await?;
            let checksum = Sha256::digest(&content);

            // 4. Parse CSV/XML/PDF format
            let records = self.parse_settlement_file(&content, &file.format)?;

            // 5. Log to audit trail
            self.audit_log(SettlementFileIngested {
                filename: file.name,
                checksum: hex::encode(checksum),
                record_count: records.len(),
            }).await?;

            all_records.extend(records);
        }

        Ok(all_records)
    }
}
```

### 9.4 Scanned Document Settlement

```rust
pub struct ScannedSettlementHandler {
    document_service: DocumentServiceClient,
    ai_gateway: AiGatewayClient,
}

impl ScannedSettlementHandler {
    pub async fn extract_from_pdf(&self, pdf_bytes: &[u8]) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // 1. Upload to document-service
        let doc = self.document_service.upload(pdf_bytes, "settlement_advice").await?;

        // 2. Trigger OCR via ai-gateway (Qwen3-VL 8B)
        let extraction = self.ai_gateway.extract_settlement(doc.id).await?;

        // 3. Parse extracted structured data into RawSettlementRecords
        let records = self.parse_extracted_data(extraction)?;

        Ok(records)
    }
}
```

---

## 10. Bulkhead Isolation

```rust
pub struct AdapterBulkhead {
    client: reqwest::Client,
    pool_size: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl AdapterBulkhead {
    pub fn new(config: &BulkheadConfig) -> Self {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(config.pool_size)
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            pool_size: config.pool_size,
            connect_timeout: config.connect_timeout,
            request_timeout: config.request_timeout,
        }
    }
}

// Each adapter has its own BulkheadConfig:
// - authorize/capture: pool_size=10, timeout=10s
// - settlement polling: pool_size=2, timeout=30s
// - webhook delivery: pool_size=5, timeout=10s
```

---

## 11. Conformance Test Suite

```rust
#[cfg(test)]
mod conformance_tests {
    /// Shared conformance suite — every connector must pass 100%
    /// Run against connector's sandbox environment

    #[tokio::test]
    async fn test_authorize_approved() {
        let connector = get_sandbox_connector();
        let result = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        assert_eq!(result.status, AuthorizeStatus::Approved);
        assert!(result.acquirer_reference.is_some());
    }

    #[tokio::test]
    async fn test_authorize_declined_insufficient_funds() {
        let connector = get_sandbox_connector();
        let result = connector.authorize(AuthorizeRequest::test_declined("insufficient_funds")).await.unwrap();
        assert_eq!(result.status, AuthorizeStatus::Declined);
        assert_eq!(result.decline_reason, Some(DeclineReason::InsufficientFunds));
    }

    #[tokio::test]
    async fn test_capture_full() {
        let connector = get_sandbox_connector();
        let auth = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        let result = connector.capture(CaptureRequest {
            acquirer_reference: auth.acquirer_reference.unwrap(),
            amount: Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() },
        }).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_void() {
        let connector = get_sandbox_connector();
        let auth = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        let result = connector.void(VoidRequest {
            acquirer_reference: auth.acquirer_reference.unwrap(),
        }).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_refund_full() {
        let connector = get_sandbox_connector();
        let auth = connector.authorize(AuthorizeRequest::test_approved()).await.unwrap();
        connector.capture(CaptureRequest { ... }).await.unwrap();
        let result = connector.refund(RefundRequest {
            acquirer_reference: auth.acquirer_reference.unwrap(),
            amount: Money { amount_minor_units: 10000, currency: CurrencyCode::new("AED").unwrap() },
        }).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let connector = get_sandbox_connector().with_slow_network(Duration::from_secs(20));
        let result = connector.authorize(AuthorizeRequest::test_approved()).await;
        assert!(matches!(result, Err(ConnectorError::Timeout)));
    }

    #[tokio::test]
    async fn test_webhook_signature_valid() {
        let connector = get_sandbox_connector();
        let (headers, body) = connector.create_test_webhook();
        assert!(connector.verify_webhook_signature(&headers, &body).is_ok());
    }

    #[tokio::test]
    async fn test_webhook_signature_tampered() {
        let connector = get_sandbox_connector();
        let (_, body) = connector.create_test_webhook();
        let mut headers = HeaderMap::new();
        headers.insert("X-Signature", "tampered".parse().unwrap());
        assert!(connector.verify_webhook_signature(&headers, &body).is_err());
    }

    #[tokio::test]
    async fn test_idempotency_native() {
        if !connector.supports_native_idempotency() { return; }
        let connector = get_sandbox_connector();
        let idem_key = uuid::Uuid::now_v7().to_string();
        let r1 = connector.authorize(AuthorizeRequest::test_with_idem(&idem_key)).await.unwrap();
        let r2 = connector.authorize(AuthorizeRequest::test_with_idem(&idem_key)).await.unwrap();
        assert_eq!(r1.acquirer_reference, r2.acquirer_reference);
    }

    #[tokio::test]
    async fn test_idempotency_status_check() {
        if connector.supports_native_idempotency() { return; }
        let connector = get_sandbox_connector();
        let idem_key = uuid::Uuid::now_v7().to_string();
        let r1 = connector.authorize(AuthorizeRequest::test_with_idem(&idem_key)).await.unwrap();
        let r2 = connector.status_check(StatusCheckRequest {
            acquirer_reference: r1.acquirer_reference.unwrap(),
        }).await.unwrap();
        assert!(r2.status != AuthorizeStatus::Unknown);
    }
}
```

---

## 12. Card Scheme Compliance Monitoring

```rust
pub struct SchemeComplianceMonitor {
    analytics_db: ClickHouseClient,
    notification_service: NotificationClient,
}

impl SchemeComplianceMonitor {
    pub async fn check_compliance(&self) -> Result<Vec<ComplianceAlert>, PlatformError> {
        let mut alerts = vec![];

        // Visa chargeback ratio (30-day rolling)
        let visa_cb = self.analytics_db.query_chargeback_ratio("visa", 30).await?;
        if visa_cb > 0.009 {
            alerts.push(ComplianceAlert {
                scheme: "Visa".into(),
                metric: "chargeback_ratio".into(),
                current: visa_cb,
                threshold: 0.01,
                severity: if visa_cb > 0.01 { "critical" } else { "warning" },
            });
        }

        // Mastercard chargeback ratio (30-day rolling)
        let mc_cb = self.analytics_db.query_chargeback_ratio("mastercard", 30).await?;
        if mc_cb > 0.0135 {
            alerts.push(ComplianceAlert {
                scheme: "Mastercard".into(),
                metric: "chargeback_ratio".into(),
                current: mc_cb,
                threshold: 0.015,
                severity: if mc_cb > 0.015 { "critical" } else { "warning" },
            });
        }

        // Send alerts
        for alert in &alerts {
            self.notification_service.send_compliance_alert(alert).await?;
        }

        Ok(alerts)
    }
}
```

---

## 13. Connector-Specific Implementation Examples

### Network International (UAE Regional Acquirer)

```rust
pub struct NetworkInternationalConnector {
    api_key: String,
    merchant_id: String,
    environment: String, // 'sandbox' | 'production'
    base_url: String,
    bulkhead: AdapterBulkhead,
    circuit_breaker: CircuitBreaker,
    decline_table: DeclineMappingTable,
}

#[async_trait]
impl AcquirerConnector for NetworkInternationalConnector {
    fn connector_id(&self) -> ConnectorId { "network_international".into() }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: false, // use status-check
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            supported_currencies: vec![CurrencyCode::new("AED").unwrap()],
            settlement_format: SettlementFormat::Webhook,
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        // 1. Build NI-specific request
        let ni_req = self.build_authorize_request(&req)?;

        // 2. Call NI API via bulkhead-isolated client
        let ni_resp = self.bulkhead.post(&format!("{}/api/v1/authorize", self.base_url), &ni_req).await?;

        // 3. Normalize response
        let status = self.classify_status(&ni_resp);
        let decline = ni_resp.response_code.as_ref()
            .map(|code| self.decline_table.normalize(code));

        Ok(AuthorizeResponse {
            status,
            acquirer_reference: ni_resp.transaction_id,
            decline_reason: decline,
            approved_amount: ni_resp.approved_amount.map(|a| Money { amount_minor_units: a, currency: req.amount.currency }),
            three_ds_data: ni_resp.three_ds,
            latency_ms: ni_resp.latency_ms,
        })
    }

    fn onboarding_schema(&self) -> OnboardingSchema {
        OnboardingSchema {
            connector_id: "network_international".into(),
            fields: vec![
                OnboardingField { name: "api_key".into(), field_type: FieldType::Password, required: true, label: "API Key".into(), validation_regex: None, help_text: None },
                OnboardingField { name: "merchant_id".into(), field_type: FieldType::String, required: true, label: "Merchant ID".into(), validation_regex: None, help_text: None },
                OnboardingField { name: "environment".into(), field_type: FieldType::Select { options: vec![
                    SelectOption { value: "sandbox".into(), label: "Sandbox".into() },
                    SelectOption { value: "production".into(), label: "Production".into() },
                ]}, required: true, label: "Environment".into(), validation_regex: None, help_text: None },
            ],
        }
    }

    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<(), ConnectorError> {
        // CRED-003: sandbox/status-check call only, never live-money
        let resp = self.bulkhead.get(&format!("{}/api/v1/merchant/status", self.base_url)).await?;
        if resp.status_code != 200 {
            return Err(ConnectorError::AuthenticationFailed);
        }
        Ok(())
    }
}
```

### Checkout.com (International PSP)

```rust
pub struct CheckoutComConnector {
    secret_key: String,
    environment: String,
    base_url: String,
    bulkhead: AdapterBulkhead,
    circuit_breaker: CircuitBreaker,
    decline_table: DeclineMappingTable,
}

#[async_trait]
impl AcquirerConnector for CheckoutComConnector {
    fn connector_id(&self) -> ConnectorId { "checkout_com".into() }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: true,
            supports_partial_refund: true,
            supports_native_idempotency_key: true, // Checkout.com has idempotency keys
            supports_webhook_settlement: true,
            supports_realtime_status_check: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex],
            supported_currencies: vec![
                CurrencyCode::new("AED").unwrap(),
                CurrencyCode::new("USD").unwrap(),
                CurrencyCode::new("EUR").unwrap(),
                CurrencyCode::new("GBP").unwrap(),
            ],
            settlement_format: SettlementFormat::Webhook,
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let cc_req = self.build_authorize_request(&req)?;
        let cc_resp = self.bulkhead.post(&format!("{}/payments", self.base_url), &cc_req).await?;
        Ok(self.normalize_authorize_response(&cc_resp)?)
    }

    // Checkout.com uses native idempotency keys
    // No status-check-before-retry needed for this connector
}
```

### Telr (Regional PSP)

```rust
pub struct TelrConnector {
    store_id: String,
    api_key: String,
    environment: String,
    base_url: String,
    bulkhead: AdapterBulkhead,
    circuit_breaker: CircuitBreaker,
    decline_table: DeclineMappingTable,
}

#[async_trait]
impl AcquirerConnector for TelrConnector {
    fn connector_id(&self) -> ConnectorId { "telr".into() }

    fn capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_partial_capture: false, // Telr doesn't support partial capture
            supports_partial_refund: true,
            supports_native_idempotency_key: false,
            supports_webhook_settlement: false, // Polling API only
            supports_realtime_status_check: true,
            supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            supported_currencies: vec![CurrencyCode::new("AED").unwrap(), CurrencyCode::new("USD").unwrap()],
            settlement_format: SettlementFormat::PollingApi,
        }
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
        let telr_req = self.build_authorize_request(&req)?;
        let telr_resp = self.bulkhead.post(&format!("{}/api/v2/order/preauth", self.base_url), &telr_req).await?;
        Ok(self.normalize_authorize_response(&telr_resp)?)
    }

    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
        // Telr uses polling API for settlement
        let resp = self.bulkhead.get(&format!(
            "{}/api/v2/order/list?since={}", self.base_url, req.since.unwrap_or_default()
        )).await?;
        self.parse_settlement_list(&resp)
    }
}
```

---

## 14. TDD Tests (Extended)

```rust
#[tokio::test]
async fn test_authorize_approved() {
    let connector = MockConnector::new().with_response(AuthorizeResponse::approved());
    let result = connector.authorize(AuthorizeRequest { ... }).await.unwrap();
    assert_eq!(result.status, AuthorizeStatus::Approved);
}

#[tokio::test]
async fn test_authorize_declined() {
    let connector = MockConnector::new().with_response(AuthorizeResponse::declined(DeclineReason::InsufficientFunds));
    let result = connector.authorize(AuthorizeRequest { ... }).await.unwrap();
    assert_eq!(result.status, AuthorizeStatus::Declined);
    assert_eq!(result.decline_reason, Some(DeclineReason::InsufficientFunds));
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_high_error_rate() {
    let connector = MockConnector::new().with_failing_rate(0.6); // 60% failure
    // Send 10 requests — circuit should open after error threshold exceeded
    for _ in 0..10 {
        connector.authorize(AuthorizeRequest { ... }).await.ok();
    }
    assert!(connector.circuit_breaker().is_open());
}

#[tokio::test]
async fn test_circuit_breaker_half_open_after_timeout() {
    let connector = MockConnector::new().with_failing_rate(1.0);
    // Trigger circuit open
    for _ in 0..10 {
        connector.authorize(AuthorizeRequest { ... }).await.ok();
    }
    // Wait for open duration
    tokio::time::sleep(Duration::from_secs(61)).await;
    assert!(connector.circuit_breaker().is_half_open());
}

#[tokio::test]
async fn test_webhook_signature_verification() {
    let connector = MockConnector::new();
    let headers = HeaderMap::new();
    let body = b"test payload";
    let valid_sig = connector.compute_signature(body);
    headers.insert("X-Signature", valid_sig.parse().unwrap());
    assert!(connector.verify_webhook_signature(&headers, body).is_ok());
}

#[tokio::test]
async fn test_webhook_signature_tampered_rejected() {
    let connector = MockConnector::new();
    let headers = HeaderMap::new();
    let body = b"test payload";
    headers.insert("X-Signature", "tampered".parse().unwrap());
    assert!(connector.verify_webhook_signature(&headers, body).is_err());
}
```
