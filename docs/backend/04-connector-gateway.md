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

## 6. TDD Tests

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
