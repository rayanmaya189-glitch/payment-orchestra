//! Connector-gateway domain model.
//! Anti-Corruption Layer — normalizes N acquirer APIs into one protocol.
//! Includes: AcquirerConnector trait, CircuitBreaker, GatewayProfile, ConnectorRegistry, mock connectors.

use std::collections::HashMap;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Re-exports ──────────────────────────────────────────────────────────────

pub use onboarding::*;

mod onboarding;

// ─── Error Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConnectorError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Acquirer declined: {0}")]
    AcquirerDeclined(String),
    #[error("Timeout after {0}ms")]
    Timeout(u64),
    #[error("Circuit breaker open for {0}")]
    CircuitBreakerOpen(String),
    #[error("Rate limited by acquirer")]
    RateLimited,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("Network error: {0}")]
    NetworkError(String),
}

// ─── AcquirerConnector Trait ─────────────────────────────────────────────────

#[async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> &str;
    fn capabilities(&self) -> ConnectorCapabilities;
    fn onboarding_schema(&self) -> OnboardingSchema;

    // Core payment operations
    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError>;
    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError>;
    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError>;
    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError>;
    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError>;

    // FX & Settlement
    async fn get_fx_rate(&self, req: FxRateRequest) -> Result<FxRateResponse, ConnectorError>;
    fn settlement_cycle(&self) -> SettlementCycle;

    // 3D Secure
    async fn check_3ds_enrollment(&self, req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError>;
    async fn authenticate_3ds(&self, req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError>;

    // Network Token
    async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError>;
    async fn account_updater(&self, token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError>;

    // Settlement & Webhooks
    async fn poll_settlement(&self, req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError>;
    fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError>;
    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError>;

    // BYOK: Credential Validation
    async fn validate_credentials(&self, config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError>;
    async fn test_connection(&self, config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError>;
    fn test_card_numbers(&self) -> Vec<TestCardNumber>;
}

// ─── Connector Capabilities ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCapabilities {
    pub supports_partial_capture: bool,
    pub supports_partial_refund: bool,
    pub supports_native_idempotency_key: bool,
    pub supports_webhook_settlement: bool,
    pub supports_realtime_status_check: bool,
    pub supports_fx_conversion: bool,
    pub supported_card_schemes: Vec<CardScheme>,
    pub supported_currencies: Vec<String>,
    pub settlement_format: SettlementFormat,
    pub settlement_cycle: SettlementCycle,
    pub cross_border_fee_bps: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CardScheme {
    Visa,
    Mastercard,
    Amex,
    Other(String),
}

impl std::fmt::Display for CardScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardScheme::Visa => write!(f, "visa"),
            CardScheme::Mastercard => write!(f, "mastercard"),
            CardScheme::Amex => write!(f, "amex"),
            CardScheme::Other(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementCycle {
    SameDay,
    NextDay,
    TwoDays,
    ThreeDays,
    Weekly,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementFormat {
    Webhook,
    PollingApi,
    Sftp,
    ScannedDocument,
}

// ─── Normalized Request/Response Types ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    pub payment_method_token: String,
    pub amount: Money,
    pub currency: String,
    pub idempotency_key: String,
    pub card_scheme: CardScheme,
    pub metadata: Option<serde_json::Value>,
    pub three_ds_data: Option<ThreeDsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeResponse {
    pub status: AuthorizeStatus,
    pub acquirer_reference: Option<String>,
    pub decline_reason: Option<String>,
    pub approved_amount: Option<Money>,
    pub three_ds_data: Option<ThreeDsData>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthorizeStatus {
    Approved,
    Declined,
    Requires3DS,
    PartialApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub acquirer_reference: String,
    pub amount: Money,
    pub currency: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResponse {
    pub success: bool,
    pub acquirer_reference: Option<String>,
    pub amount_captured: Money,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidRequest {
    pub acquirer_reference: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidResponse {
    pub success: bool,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequest {
    pub acquirer_reference: String,
    pub amount: Money,
    pub currency: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub success: bool,
    pub acquirer_reference: Option<String>,
    pub refund_id: Option<String>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCheckRequest {
    pub acquirer_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCheckResponse {
    pub status: AuthorizeStatus,
    pub acquirer_reference: Option<String>,
    pub amount: Option<Money>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor_units: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDsData {
    pub three_ds_version: String,
    pub acs_url: Option<String>,
    pub pareq: Option<String>,
    pub md: Option<String>,
    pub session_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRateRequest {
    pub source_currency: String,
    pub target_currency: String,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRateResponse {
    pub rate: String,
    pub rate_minor_units: i64,
    pub converted_amount: Money,
    pub fee: Option<Money>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check3dsRequest {
    pub card_number: String,
    pub amount: Money,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check3dsResponse {
    pub requires_3ds: bool,
    pub three_ds_data: Option<ThreeDsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authenticate3dsRequest {
    pub three_ds_data: ThreeDsData,
    pub authentication_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authenticate3dsResponse {
    pub authenticated: bool,
    pub three_ds_status: String,
    pub eci: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionTokenRequest {
    pub card_number: String,
    pub expiry_month: u32,
    pub expiry_year: u32,
    pub cardholder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionTokenResponse {
    pub network_token: String,
    pub token_expiry_month: u32,
    pub token_expiry_year: u32,
    pub cryptogram: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTokenReference {
    pub network_token: String,
    pub connector_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdateResult {
    pub updated: bool,
    pub new_expiry_month: Option<u32>,
    pub new_expiry_year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollSettlementRequest {
    pub since: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSettlementRecord {
    pub transaction_id: String,
    pub amount: Money,
    pub fee: Option<Money>,
    pub settlement_date: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub api_key: Option<String>,
    pub secret_key: Option<String>,
    pub merchant_id: Option<String>,
    pub store_id: Option<String>,
    pub environment: String,
    pub additional_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialValidationResult {
    pub valid: bool,
    pub merchant_name: Option<String>,
    pub permissions: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub merchant_name: Option<String>,
    pub latency_ms: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCardNumber {
    pub label: String,
    pub card_number: String,
    pub scheme: CardScheme,
    pub scenario: String,
}

// ─── Gateway Profile ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProfile {
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub status: ProfileStatus,
    pub limits: TransactionLimits,
    pub fees: FeeStructure,
    pub routing_priority: i32,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<String>,
    pub enabled_countries: Vec<String>,
    pub rate_limits: RateLimitConfig,
    pub monitoring: MonitoringThresholds,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileStatus {
    Active,
    Disabled,
    Maintenance,
}

impl ProfileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileStatus::Active => "active",
            ProfileStatus::Disabled => "disabled",
            ProfileStatus::Maintenance => "maintenance",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLimits {
    pub min_amount_minor: i64,
    pub max_amount_minor: i64,
    pub daily_volume_limit_minor: i64,
    pub monthly_volume_limit_minor: i64,
    pub max_refund_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub fixed_fee_minor: i64,
    pub percentage_fee_bps: i32,
    pub cross_border_fee_bps: i32,
    pub currency_conversion_fee_bps: i32,
    pub max_fee_cap: Option<i64>,
    pub min_fee_floor: Option<i64>,
    pub tiered_pricing: Option<Vec<FeeTier>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTier {
    pub min_volume_minor: i64,
    pub max_volume_minor: Option<i64>,
    pub percentage_fee_bps: i32,
}

impl FeeStructure {
    pub fn calculate_fee(&self, amount: &Money, is_cross_border: bool, requires_fx: bool, daily_volume: i64) -> Money {
        let percentage_fee = (amount.amount_minor_units * self.percentage_fee_bps as i64) / 10000;
        let cross_border = if is_cross_border {
            (amount.amount_minor_units * self.cross_border_fee_bps as i64) / 10000
        } else {
            0
        };
        let fx_fee = if requires_fx {
            (amount.amount_minor_units * self.currency_conversion_fee_bps as i64) / 10000
        } else {
            0
        };

        let mut total = self.fixed_fee_minor + percentage_fee + cross_border + fx_fee;

        // Apply tiered pricing if configured
        if let Some(tiers) = &self.tiered_pricing {
            if let Some(tier) = tiers.iter().find(|t| {
                daily_volume >= t.min_volume_minor
                    && t.max_volume_minor.map_or(true, |max| daily_volume < max)
            }) {
                total = self.fixed_fee_minor
                    + (amount.amount_minor_units * tier.percentage_fee_bps as i64) / 10000
                    + cross_border
                    + fx_fee;
            }
        }

        // Apply fee cap
        if let Some(cap) = self.max_fee_cap {
            total = total.min(cap);
        }

        // Apply fee floor
        if let Some(floor) = self.min_fee_floor {
            total = total.max(floor);
        }

        Money {
            amount_minor_units: total,
            currency: amount.currency.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub per_second: u32,
    pub per_day: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringThresholds {
    pub success_rate_alert: f64,
    pub success_rate_critical: f64,
    pub latency_p99_alert_ms: u32,
    pub latency_p99_critical_ms: u32,
    pub auto_disable_on_low_success: bool,
}

// ─── Circuit Breaker ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    error_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    open_duration: std::time::Duration,
    error_threshold: f64,
    window: std::time::Duration,
    half_open_max_requests: u32,
    half_open_requests: u32,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            error_count: 0,
            success_count: 0,
            last_failure: None,
            open_duration: std::time::Duration::from_secs(60),
            error_threshold: 0.5,
            window: std::time::Duration::from_secs(30),
            half_open_max_requests: 3,
            half_open_requests: 0,
        }
    }

    pub fn state(&self) -> &CircuitState {
        &self.state
    }

    pub fn is_call_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if open duration has elapsed → transition to HalfOpen
                if let Some(last_fail) = self.last_failure {
                    if last_fail.elapsed() >= self.open_duration {
                        self.state = CircuitState::HalfOpen;
                        self.half_open_requests = 0;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                if self.half_open_requests < self.half_open_max_requests {
                    self.half_open_requests += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        match self.state {
            CircuitState::HalfOpen => {
                // Enough successful requests → close the circuit
                self.state = CircuitState::Closed;
                self.error_count = 0;
                self.success_count = 0;
                self.half_open_requests = 0;
            }
            CircuitState::Closed => {
                // Periodically reset counts to avoid stale data
                let total = self.error_count + self.success_count;
                if total > 100 {
                    self.error_count = 0;
                    self.success_count = 0;
                }
            }
            _ => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.error_count += 1;
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                let total = self.error_count + self.success_count;
                if total > 0 {
                    let error_rate = self.error_count as f64 / total as f64;
                    if error_rate >= self.error_threshold {
                        self.state = CircuitState::Open;
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Failure in HalfOpen → back to Open
                self.state = CircuitState::Open;
                self.half_open_requests = 0;
            }
            _ => {}
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Connector Registry ──────────────────────────────────────────────────────

pub struct ConnectorRegistry {
    connectors: HashMap<String, Box<dyn AcquirerConnector>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn register(&mut self, connector: Box<dyn AcquirerConnector>) {
        let id = connector.connector_id().to_string();
        self.connectors.insert(id, connector);
    }

    pub fn get(&self, connector_id: &str) -> Result<&dyn AcquirerConnector, ConnectorError> {
        self.connectors
            .get(connector_id)
            .map(|c| c.as_ref())
            .ok_or_else(|| ConnectorError::InvalidRequest(format!("Connector '{}' not found", connector_id)))
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.connectors.keys().cloned().collect()
    }

    pub fn list_all(&self) -> Vec<&dyn AcquirerConnector> {
        self.connectors.values().map(|c| c.as_ref()).collect()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Decline Code Normalization ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DeclineMappingTable {
    mappings: HashMap<String, String>,
}

impl DeclineMappingTable {
    pub fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }

    pub fn normalize(&self, raw_code: &str) -> String {
        self.mappings
            .get(raw_code)
            .cloned()
            .unwrap_or_else(|| format!("UnknownError({})", raw_code))
    }
}

impl Default for DeclineMappingTable {
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert("insufficient_funds".into(), "InsufficientFunds".into());
        m.insert("do_not_honor".into(), "DoNotHonor".into());
        m.insert("invalid_card_number".into(), "InvalidCard".into());
        m.insert("expired_card".into(), "ExpiredCard".into());
        m.insert("card_declined".into(), "SuspectedFraud".into());
        m.insert("gateway_timeout".into(), "IssuerUnavailable".into());
        m.insert("3ds_failed".into(), "ThreeDSecureFailed".into());
        m.insert("rate_limit_exceeded".into(), "RateLimitedByAcquirer".into());
        Self { mappings: m }
    }
}

// ─── Rotation Strategy ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    Priority,
    RoundRobin,
    WeightedRoundRobin { weights: Vec<(Uuid, u32)> },
    CostBased,
    SuccessRateBased,
    VolumeCapped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationState {
    pub operator_id: Uuid,
    pub strategy: RotationStrategy,
    pub current_index: u32,
    pub last_used_gateway_id: Option<Uuid>,
    pub weights: Vec<(Uuid, u32)>,
    pub daily_volume: HashMap<Uuid, i64>,
}

impl RotationState {
    pub fn select_gateway_profile(
        &self,
        profiles: &[GatewayProfile],
        amount: &Money,
        card_scheme: &CardScheme,
        currency: &str,
    ) -> Result<Uuid, ConnectorError> {
        let eligible: Vec<&GatewayProfile> = profiles
            .iter()
            .filter(|p| p.status == ProfileStatus::Active)
            .filter(|p| p.enabled_card_schemes.contains(card_scheme))
            .filter(|p| p.enabled_currencies.iter().any(|c| c == currency))
            .filter(|p| amount.amount_minor_units >= p.limits.min_amount_minor)
            .filter(|p| amount.amount_minor_units <= p.limits.max_amount_minor)
            .filter(|p| {
                let daily = self.daily_volume.get(&p.profile_id).copied().unwrap_or(0);
                daily < p.limits.daily_volume_limit_minor
            })
            .collect();

        if eligible.is_empty() {
            return Err(ConnectorError::InvalidRequest("No eligible gateway profile found".into()));
        }

        match &self.strategy {
            RotationStrategy::Priority => Ok(eligible[0].profile_id),
            RotationStrategy::RoundRobin => {
                let idx = (self.current_index as usize) % eligible.len();
                Ok(eligible[idx].profile_id)
            }
            RotationStrategy::WeightedRoundRobin { weights } => {
                let total_weight: u32 = weights.iter().map(|(_, w)| w).sum();
                if total_weight == 0 {
                    return Ok(eligible[0].profile_id);
                }
                let mut random = rand::thread_rng().gen_range(0..total_weight);
                for (gateway_id, weight) in weights {
                    if random < *weight {
                        return Ok(*gateway_id);
                    }
                    random = random.saturating_sub(*weight);
                }
                Ok(eligible[0].profile_id) // fallback
            }
            RotationStrategy::CostBased => {                    let mut scored: Vec<(Uuid, i64)> = eligible
                        .iter()
                        .map(|p| {
                            let fee = p.fees.calculate_fee(amount, false, false, 0);
                            (p.profile_id, fee.amount_minor_units)
                        })
                        .collect();                    scored.sort_by_key(|(_, fee)| *fee);
                    Ok(scored[0].0)
            }
            RotationStrategy::SuccessRateBased => {
                // Without real success rates, fall back to priority
                Ok(eligible[0].profile_id)
            }
            RotationStrategy::VolumeCapped => {
                for profile in &eligible {
                    let daily = self.daily_volume.get(&profile.profile_id).copied().unwrap_or(0);
                    if daily < profile.limits.daily_volume_limit_minor {
                        return Ok(profile.profile_id);
                    }
                }
                Err(ConnectorError::InvalidRequest("All gateways exceeded volume limits".into()))
            }
        }
    }
}

// ─── Validation Function ─────────────────────────────────────────────────────

pub fn validate_transaction_against_profile(
    amount: &Money,
    profile: &GatewayProfile,
    card_scheme: &CardScheme,
    currency: &str,
) -> Result<(), ConnectorError> {
    if amount.amount_minor_units < profile.limits.min_amount_minor {
        return Err(ConnectorError::InvalidRequest("Below minimum amount".into()));
    }
    if amount.amount_minor_units > profile.limits.max_amount_minor {
        return Err(ConnectorError::InvalidRequest("Exceeds maximum amount".into()));
    }
    if !profile.enabled_card_schemes.contains(card_scheme) {
        return Err(ConnectorError::InvalidRequest("Unsupported card scheme".into()));
    }
    if !profile.enabled_currencies.iter().any(|c| c == currency) {
        return Err(ConnectorError::InvalidRequest("Unsupported currency".into()));
    }
    Ok(())
}

// ─── Gateway Profile Error Catalog ───────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum GatewayError {
    #[error("Gateway profile not found: {0}")]
    NotFound(Uuid),
    #[error("Transaction below gateway minimum amount")]
    BelowMinimumAmount,
    #[error("Transaction exceeds gateway maximum amount")]
    ExceedsMaximumAmount,
    #[error("Daily volume limit reached")]
    DailyVolumeExceeded,
    #[error("Monthly volume limit reached")]
    MonthlyVolumeExceeded,
    #[error("Card scheme not enabled for this gateway")]
    UnsupportedCardScheme,
    #[error("Currency not enabled for this gateway")]
    UnsupportedCurrency,
    #[error("Gateway profile is disabled")]
    GatewayDisabled,
    #[error("Gateway in maintenance mode")]
    GatewayMaintenance,
    #[error("All gateways hit daily volume limit")]
    AllGatewaysVolumeExceeded,
    #[error("Unknown rotation strategy")]
    RotationStrategyInvalid,
}

// ─── Mock Connector Implementations ──────────────────────────────────────────

pub mod mocks {
    use super::*;

    // ── Network International Mock (UAE Regional Acquirer) ──────────────

    pub struct MockNetworkIntlConnector {
        circuit_breaker: std::sync::Mutex<CircuitBreaker>,
        decline_table: DeclineMappingTable,
        environment: String,
    }

    impl MockNetworkIntlConnector {
        pub fn new(environment: &str) -> Self {
            Self {
                circuit_breaker: std::sync::Mutex::new(CircuitBreaker::new()),
                decline_table: DeclineMappingTable::default(),
                environment: environment.to_string(),
            }
        }
    }

    #[async_trait]
    impl AcquirerConnector for MockNetworkIntlConnector {
        fn connector_id(&self) -> &str {
            "network_international"
        }

        fn capabilities(&self) -> ConnectorCapabilities {
            ConnectorCapabilities {
                supports_partial_capture: true,
                supports_partial_refund: true,
                supports_native_idempotency_key: false,
                supports_webhook_settlement: true,
                supports_realtime_status_check: true,
                supports_fx_conversion: false,
                supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
                supported_currencies: vec!["AED".into()],
                settlement_format: SettlementFormat::Webhook,
                settlement_cycle: SettlementCycle::SameDay,
                cross_border_fee_bps: 50,
            }
        }

        fn onboarding_schema(&self) -> OnboardingSchema {
            OnboardingSchema {
                connector_id: "network_international".into(),
                fields: vec![
                    OnboardingField {
                        name: "api_key".into(),
                        field_type: FieldType::Password,
                        required: true,
                        label: "API Key".into(),
                        validation_regex: None,
                        help_text: None,
                    },
                    OnboardingField {
                        name: "merchant_id".into(),
                        field_type: FieldType::String,
                        required: true,
                        label: "Merchant ID".into(),
                        validation_regex: None,
                        help_text: None,
                    },
                    OnboardingField {
                        name: "environment".into(),
                        field_type: FieldType::Select {
                            options: vec![
                                SelectOption { value: "sandbox".into(), label: "Sandbox".into() },
                                SelectOption { value: "production".into(), label: "Production".into() },
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

        async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
            {
                let mut cb = self.circuit_breaker.lock().unwrap();
                if !cb.is_call_allowed() {
                    return Err(ConnectorError::CircuitBreakerOpen("network_international".into()));
                }
            }
            // Simulate sandbox approval
            if self.environment == "sandbox" {
                self.circuit_breaker.lock().unwrap().record_success();
                Ok(AuthorizeResponse {
                    status: AuthorizeStatus::Approved,
                    acquirer_reference: Some(format!("ni_auth_{}", uuid::Uuid::now_v7())),
                    decline_reason: None,
                    approved_amount: Some(req.amount),
                    three_ds_data: None,
                    latency_ms: 120,
                })
            } else {
                self.circuit_breaker.lock().unwrap().record_failure();
                Err(ConnectorError::AuthenticationFailed("production not configured in mock".into()))
            }
        }

        async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
            Ok(CaptureResponse {
                success: true,
                acquirer_reference: Some(format!("ni_cap_{}", uuid::Uuid::now_v7())),
                amount_captured: req.amount,
                latency_ms: 80,
            })
        }

        async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
            Ok(VoidResponse {
                success: true,
                acquirer_reference: Some(req.acquirer_reference),
                latency_ms: 60,
            })
        }

        async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
            Ok(RefundResponse {
                success: true,
                acquirer_reference: Some(format!("ni_ref_{}", uuid::Uuid::now_v7())),
                refund_id: Some(format!("ni_rf_{}", uuid::Uuid::now_v7())),
                latency_ms: 90,
            })
        }

        async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
            Ok(StatusCheckResponse {
                status: AuthorizeStatus::Approved,
                acquirer_reference: Some(req.acquirer_reference),
                amount: None,
                latency_ms: 50,
            })
        }

        async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("FX not supported by Network International".into()))
        }

        fn settlement_cycle(&self) -> SettlementCycle {
            SettlementCycle::SameDay
        }

        async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
            Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None })
        }

        async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("3DS not implemented".into()))
        }

        async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("Network tokens not supported".into()))
        }

        async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("Account updater not supported".into()))
        }

        async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
            Ok(vec![])
        }

        fn verify_webhook_signature(&self, headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
            if headers.get("x-signature").map(|s| s == "valid_sig").unwrap_or(false) {
                Ok(())
            } else {
                Err(ConnectorError::InvalidSignature)
            }
        }

        fn parse_webhook(&self, _body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
            Ok(ConnectorEvent {
                event_type: "payment.captured".into(),
                payload: serde_json::json!({"status": "captured"}),
            })
        }

        async fn validate_credentials(&self, _config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
            Ok(CredentialValidationResult {
                valid: true,
                merchant_name: Some("Mock Merchant NI".into()),
                permissions: vec!["authorize".into(), "capture".into(), "refund".into()],
                error_message: None,
            })
        }

        async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
            Ok(ConnectionTestResult {
                success: true,
                merchant_name: Some("Mock Merchant NI".into()),
                latency_ms: 150,
                error_message: None,
            })
        }

        fn test_card_numbers(&self) -> Vec<TestCardNumber> {
            vec![TestCardNumber {
                label: "Visa Approved".into(),
                card_number: "4111111111111111".into(),
                scheme: CardScheme::Visa,
                scenario: "authorize_approved".into(),
            }]
        }
    }

    // ── Checkout.com Mock (International PSP) ─────────────────────────

    pub struct MockCheckoutComConnector {
        environment: String,
    }

    impl MockCheckoutComConnector {
        pub fn new(environment: &str) -> Self {
            Self { environment: environment.to_string() }
        }
    }

    #[async_trait]
    impl AcquirerConnector for MockCheckoutComConnector {
        fn connector_id(&self) -> &str {
            "checkout_com"
        }

        fn capabilities(&self) -> ConnectorCapabilities {
            ConnectorCapabilities {
                supports_partial_capture: true,
                supports_partial_refund: true,
                supports_native_idempotency_key: true,
                supports_webhook_settlement: true,
                supports_realtime_status_check: true,
                supports_fx_conversion: true,
                supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard, CardScheme::Amex],
                supported_currencies: vec!["AED".into(), "USD".into(), "EUR".into(), "GBP".into()],
                settlement_format: SettlementFormat::Webhook,
                settlement_cycle: SettlementCycle::NextDay,
                cross_border_fee_bps: 75,
            }
        }

        fn onboarding_schema(&self) -> OnboardingSchema {
            OnboardingSchema {
                connector_id: "checkout_com".into(),
                fields: vec![
                    OnboardingField {
                        name: "secret_key".into(),
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
                                SelectOption { value: "sandbox".into(), label: "Sandbox".into() },
                                SelectOption { value: "production".into(), label: "Production".into() },
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

        async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
            Ok(AuthorizeResponse {
                status: AuthorizeStatus::Approved,
                acquirer_reference: Some(format!("cko_auth_{}", uuid::Uuid::now_v7())),
                decline_reason: None,
                approved_amount: Some(req.amount),
                three_ds_data: None,
                latency_ms: 95,
            })
        }

        async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
            Ok(CaptureResponse {
                success: true,
                acquirer_reference: Some(format!("cko_cap_{}", uuid::Uuid::now_v7())),
                amount_captured: req.amount,
                latency_ms: 65,
            })
        }

        async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
            Ok(VoidResponse {
                success: true,
                acquirer_reference: Some(req.acquirer_reference),
                latency_ms: 45,
            })
        }

        async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
            Ok(RefundResponse {
                success: true,
                acquirer_reference: Some(format!("cko_ref_{}", uuid::Uuid::now_v7())),
                refund_id: Some(format!("cko_rf_{}", uuid::Uuid::now_v7())),
                latency_ms: 70,
            })
        }

        async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
            Ok(StatusCheckResponse {
                status: AuthorizeStatus::Approved,
                acquirer_reference: Some(req.acquirer_reference),
                amount: None,
                latency_ms: 40,
            })
        }

        async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
            Ok(FxRateResponse {
                rate: "3.6725".into(),
                rate_minor_units: 3672500,
                converted_amount: Money { amount_minor_units: 10000, currency: "AED".into() },
                fee: Some(Money { amount_minor_units: 25, currency: "AED".into() }),
                expires_at: Utc::now() + chrono::Duration::minutes(5),
            })
        }

        fn settlement_cycle(&self) -> SettlementCycle {
            SettlementCycle::NextDay
        }

        async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
            Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None })
        }

        async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("3DS not implemented".into()))
        }

        async fn provision_network_token(&self, req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
            Ok(ProvisionTokenResponse {
                network_token: format!("cko_tok_{}", uuid::Uuid::now_v7()),
                token_expiry_month: req.expiry_month,
                token_expiry_year: req.expiry_year + 1,
                cryptogram: Some("AEBB0C4C7A4D3B".into()),
            })
        }

        async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
            Ok(AccountUpdateResult {
                updated: true,
                new_expiry_month: Some(12),
                new_expiry_year: Some(2028),
            })
        }

        async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
            Ok(vec![])
        }

        fn verify_webhook_signature(&self, headers: &HashMap<String, String>, body: &[u8]) -> Result<(), ConnectorError> {
            let expected = format!("sha256={}", hex::encode(sha256(body)));
            if headers.get("x-signature").map(|s| s == &expected).unwrap_or(false) {
                Ok(())
            } else {
                Err(ConnectorError::InvalidSignature)
            }
        }

        fn parse_webhook(&self, _body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
            Ok(ConnectorEvent {
                event_type: "payment.captured".into(),
                payload: serde_json::json!({"status": "captured"}),
            })
        }

        async fn validate_credentials(&self, _config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
            Ok(CredentialValidationResult {
                valid: true,
                merchant_name: Some("Mock Merchant CKO".into()),
                permissions: vec!["authorize".into(), "capture".into()],
                error_message: None,
            })
        }

        async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
            Ok(ConnectionTestResult {
                success: true,
                merchant_name: Some("Mock Merchant CKO".into()),
                latency_ms: 130,
                error_message: None,
            })
        }

        fn test_card_numbers(&self) -> Vec<TestCardNumber> {
            vec![
                TestCardNumber {
                    label: "Visa Approved".into(),
                    card_number: "4242424242424242".into(),
                    scheme: CardScheme::Visa,
                    scenario: "authorize_approved".into(),
                },
                TestCardNumber {
                    label: "Mastercard 3DS".into(),
                    card_number: "5200000000000007".into(),
                    scheme: CardScheme::Mastercard,
                    scenario: "requires_3ds".into(),
                },
            ]
        }
    }

    // ── Telr Mock (Regional PSP) ──────────────────────────────────────

    pub struct MockTelrConnector {
        environment: String,
    }

    impl MockTelrConnector {
        pub fn new(environment: &str) -> Self {
            Self { environment: environment.to_string() }
        }
    }

    #[async_trait]
    impl AcquirerConnector for MockTelrConnector {
        fn connector_id(&self) -> &str {
            "telr"
        }

        fn capabilities(&self) -> ConnectorCapabilities {
            ConnectorCapabilities {
                supports_partial_capture: false,
                supports_partial_refund: true,
                supports_native_idempotency_key: false,
                supports_webhook_settlement: false,
                supports_realtime_status_check: true,
                supports_fx_conversion: false,
                supported_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
                supported_currencies: vec!["AED".into(), "USD".into()],
                settlement_format: SettlementFormat::PollingApi,
                settlement_cycle: SettlementCycle::ThreeDays,
                cross_border_fee_bps: 100,
            }
        }

        fn onboarding_schema(&self) -> OnboardingSchema {
            OnboardingSchema {
                connector_id: "telr".into(),
                fields: vec![
                    OnboardingField {
                        name: "store_id".into(),
                        field_type: FieldType::String,
                        required: true,
                        label: "Store ID".into(),
                        validation_regex: None,
                        help_text: Some("Provided by Telr at onboarding".into()),
                    },
                    OnboardingField {
                        name: "api_key".into(),
                        field_type: FieldType::Password,
                        required: true,
                        label: "API Key".into(),
                        validation_regex: None,
                        help_text: None,
                    },
                    OnboardingField {
                        name: "environment".into(),
                        field_type: FieldType::Select {
                            options: vec![
                                SelectOption { value: "sandbox".into(), label: "Sandbox".into() },
                                SelectOption { value: "production".into(), label: "Production".into() },
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

        async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError> {
            Ok(AuthorizeResponse {
                status: AuthorizeStatus::Approved,
                acquirer_reference: Some(format!("telr_auth_{}", uuid::Uuid::now_v7())),
                decline_reason: None,
                approved_amount: Some(req.amount),
                three_ds_data: None,
                latency_ms: 200,
            })
        }

        async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError> {
            Ok(CaptureResponse {
                success: true,
                acquirer_reference: Some(format!("telr_cap_{}", uuid::Uuid::now_v7())),
                amount_captured: req.amount,
                latency_ms: 150,
            })
        }

        async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError> {
            Ok(VoidResponse {
                success: true,
                acquirer_reference: Some(req.acquirer_reference),
                latency_ms: 100,
            })
        }

        async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError> {
            Ok(RefundResponse {
                success: true,
                acquirer_reference: Some(format!("telr_ref_{}", uuid::Uuid::now_v7())),
                refund_id: Some(format!("telr_rf_{}", uuid::Uuid::now_v7())),
                latency_ms: 120,
            })
        }

        async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError> {
            Ok(StatusCheckResponse {
                status: AuthorizeStatus::Approved,
                acquirer_reference: Some(req.acquirer_reference),
                amount: None,
                latency_ms: 80,
            })
        }

        async fn get_fx_rate(&self, _req: FxRateRequest) -> Result<FxRateResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("FX not supported by Telr".into()))
        }

        fn settlement_cycle(&self) -> SettlementCycle {
            SettlementCycle::ThreeDays
        }

        async fn check_3ds_enrollment(&self, _req: Check3dsRequest) -> Result<Check3dsResponse, ConnectorError> {
            Ok(Check3dsResponse { requires_3ds: false, three_ds_data: None })
        }

        async fn authenticate_3ds(&self, _req: Authenticate3dsRequest) -> Result<Authenticate3dsResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("3DS not implemented".into()))
        }

        async fn provision_network_token(&self, _req: ProvisionTokenRequest) -> Result<ProvisionTokenResponse, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("Network tokens not supported".into()))
        }

        async fn account_updater(&self, _token: &NetworkTokenReference) -> Result<AccountUpdateResult, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("Account updater not supported".into()))
        }

        async fn poll_settlement(&self, _req: PollSettlementRequest) -> Result<Vec<RawSettlementRecord>, ConnectorError> {
            Ok(vec![])
        }

        fn verify_webhook_signature(&self, _headers: &HashMap<String, String>, _body: &[u8]) -> Result<(), ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("Webhooks not supported by Telr".into()))
        }

        fn parse_webhook(&self, _body: &[u8]) -> Result<ConnectorEvent, ConnectorError> {
            Err(ConnectorError::UnsupportedOperation("Webhooks not supported by Telr".into()))
        }

        async fn validate_credentials(&self, _config: &ConnectorConfig) -> Result<CredentialValidationResult, ConnectorError> {
            Ok(CredentialValidationResult {
                valid: true,
                merchant_name: Some("Mock Merchant Telr".into()),
                permissions: vec!["authorize".into(), "capture".into(), "refund".into()],
                error_message: None,
            })
        }

        async fn test_connection(&self, _config: &ConnectorConfig) -> Result<ConnectionTestResult, ConnectorError> {
            Ok(ConnectionTestResult {
                success: true,
                merchant_name: Some("Mock Merchant Telr".into()),
                latency_ms: 180,
                error_message: None,
            })
        }

        fn test_card_numbers(&self) -> Vec<TestCardNumber> {
            vec![
                TestCardNumber {
                    label: "Visa Approved".into(),
                    card_number: "4111111111111111".into(),
                    scheme: CardScheme::Visa,
                    scenario: "authorize_approved".into(),
                },
                TestCardNumber {
                    label: "Mastercard Declined".into(),
                    card_number: "5100000000000008".into(),
                    scheme: CardScheme::Mastercard,
                    scenario: "authorize_declined".into(),
                },
            ]
        }
    }
}

// ─── SHA-256 Helper ──────────────────────────────────────────────────────────

fn sha256(data: &[u8]) -> Vec<u8> {
    use ring::digest::{Context, SHA256};
    let mut ctx = Context::new(&SHA256);
    ctx.update(data);
    ctx.finish().as_ref().to_vec()
}
