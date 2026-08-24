use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::ConnectorError;
use super::types::*;

// ─── Connector Capabilities ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCapabilities {
    pub supports_partial_capture: bool,
    pub supports_partial_refund: bool,
    pub supports_native_idempotency_key: bool,
    pub supports_webhook_settlement: bool,
    pub supports_realtime_status_check: bool,
    pub supports_fx_conversion: bool,
    pub supported_card_schemes: Vec<super::types::CardScheme>,
    pub supported_currencies: Vec<String>,
    pub settlement_format: super::types::SettlementFormat,
    pub settlement_cycle: super::types::SettlementCycle,
    pub cross_border_fee_bps: i32,
}

// ─── AcquirerConnector Trait ─────────────────────────────────────────────────

#[async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> &str;
    fn capabilities(&self) -> ConnectorCapabilities;
    fn onboarding_schema(&self) -> super::onboarding::OnboardingSchema;

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
