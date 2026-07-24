//! Command types for connector-gateway.

use uuid::Uuid;

use crate::domain::{
    CardScheme, ConnectorConfig, FeeStructure, GatewayProfile, MonitoringThresholds,
    ProfileStatus, RateLimitConfig, TransactionLimits,
};
use crate::events::GatewayEvent;

// ─── Commands ────────────────────────────────────────────────────────────────

pub struct CreateGatewayProfile {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub limits: TransactionLimits,
    pub fees: FeeStructure,
    pub routing_priority: i32,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<String>,
    pub enabled_countries: Vec<String>,
    pub rate_limits: RateLimitConfig,
    pub monitoring: MonitoringThresholds,
}

pub struct UpdateGatewayProfile {
    pub profile_id: Uuid,
    pub limits: Option<TransactionLimits>,
    pub fees: Option<FeeStructure>,
    pub rate_limits: Option<RateLimitConfig>,
    pub monitoring: Option<MonitoringThresholds>,
    pub status: Option<ProfileStatus>,
    pub routing_priority: Option<i32>,
    pub enabled_card_schemes: Option<Vec<CardScheme>>,
    pub enabled_currencies: Option<Vec<String>>,
    pub enabled_countries: Option<Vec<String>>,
}

pub struct TestConnection {
    pub gateway_profile_id: Uuid,
}

pub struct ValidateCredentials {
    pub connector_id: String,
    pub config: ConnectorConfig,
}

// ─── Command Results ─────────────────────────────────────────────────────────

pub struct CreateGatewayProfileResult {
    pub profile: GatewayProfile,
    pub event: GatewayEvent,
}

pub struct UpdateGatewayProfileResult {
    pub profile: GatewayProfile,
    pub event: GatewayEvent,
}

pub struct TestConnectionResult {
    pub profile: GatewayProfile,
    pub success: bool,
    pub latency_ms: u32,
    pub event: GatewayEvent,
}

pub struct ValidateCredentialsResult {
    pub valid: bool,
    pub merchant_name: Option<String>,
    pub event: GatewayEvent,
}
