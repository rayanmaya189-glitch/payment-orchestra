//! Command handlers for connector-gateway.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::{
    self, CardScheme, ConnectorRegistry, FeeStructure, GatewayProfile, MonitoringThresholds,
    ProfileStatus, RateLimitConfig, TransactionLimits,
};
use crate::events::{ConnectionTested, CredentialsValidated, GatewayEvent, GatewayProfileCreated, GatewayProfileUpdated};
use crate::repository::GatewayProfileRepository;

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
}

pub struct TestConnection {
    pub gateway_profile_id: Uuid,
}

pub struct ValidateCredentials {
    pub connector_id: String,
    pub config: domain::ConnectorConfig,
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

// ─── Command Handler Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_gateway_profile(&self, cmd: CreateGatewayProfile) -> Result<CreateGatewayProfileResult, String>;
    async fn update_gateway_profile(&self, cmd: UpdateGatewayProfile) -> Result<UpdateGatewayProfileResult, String>;
    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, String>;
    async fn validate_credentials(&self, cmd: ValidateCredentials) -> Result<ValidateCredentialsResult, String>;
}

// ─── Handler Implementation ──────────────────────────────────────────────────

pub struct GatewayCommandHandler<R: GatewayProfileRepository> {
    repo: R,
    registry: ConnectorRegistry,
}

impl<R: GatewayProfileRepository> GatewayCommandHandler<R> {
    pub fn new(repo: R, registry: ConnectorRegistry) -> Self {
        Self { repo, registry }
    }
}

#[async_trait]
impl<R: GatewayProfileRepository + Send + Sync> CommandHandler for GatewayCommandHandler<R> {
    async fn create_gateway_profile(&self, cmd: CreateGatewayProfile) -> Result<CreateGatewayProfileResult, String> {
        let profile_id = Uuid::now_v7();

        let profile = GatewayProfile {
            profile_id,
            operator_id: cmd.operator_id,
            connector_id: cmd.connector_id.clone(),
            merchant_acquirer_link_id: cmd.merchant_acquirer_link_id,
            status: ProfileStatus::Active,
            limits: cmd.limits,
            fees: cmd.fees,
            routing_priority: cmd.routing_priority,
            enabled_card_schemes: cmd.enabled_card_schemes,
            enabled_currencies: cmd.enabled_currencies,
            enabled_countries: cmd.enabled_countries,
            rate_limits: cmd.rate_limits,
            monitoring: cmd.monitoring,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.repo.save(&profile).await?;

        let event = GatewayEvent::GatewayProfileCreated(GatewayProfileCreated {
            profile_id,
            operator_id: cmd.operator_id,
            connector_id: cmd.connector_id,
            created_at: profile.created_at,
        });

        Ok(CreateGatewayProfileResult { profile, event })
    }

    async fn update_gateway_profile(&self, cmd: UpdateGatewayProfile) -> Result<UpdateGatewayProfileResult, String> {
        let mut profile = self.repo.load(cmd.profile_id).await?.ok_or_else(|| "Gateway profile not found".to_string())?;

        if let Some(limits) = cmd.limits {
            profile.limits = limits;
        }
        if let Some(fees) = cmd.fees {
            profile.fees = fees;
        }
        if let Some(rate_limits) = cmd.rate_limits {
            profile.rate_limits = rate_limits;
        }
        if let Some(monitoring) = cmd.monitoring {
            profile.monitoring = monitoring;
        }
        if let Some(status) = cmd.status {
            profile.status = status;
        }
        if let Some(priority) = cmd.routing_priority {
            profile.routing_priority = priority;
        }
        profile.updated_at = Utc::now();

        self.repo.save(&profile).await?;

        let event = GatewayEvent::GatewayProfileUpdated(GatewayProfileUpdated {
            profile_id: cmd.profile_id,
            operator_id: profile.operator_id,
            updated_at: profile.updated_at,
        });

        Ok(UpdateGatewayProfileResult { profile, event })
    }

    async fn test_connection(&self, cmd: TestConnection) -> Result<TestConnectionResult, String> {
        let profile = self.repo.load(cmd.gateway_profile_id).await?.ok_or_else(|| "Gateway profile not found".to_string())?;

        let connector = self.registry.get(&profile.connector_id)
            .map_err(|e| format!("Connector error: {}", e))?;

        let config = domain::ConnectorConfig {
            api_key: None,
            secret_key: None,
            merchant_id: None,
            store_id: None,
            environment: "sandbox".into(),
            additional_fields: Default::default(),
        };

        let result = connector.test_connection(&config).await
            .map_err(|e| format!("Connection test failed: {}", e))?;

        let event = GatewayEvent::ConnectionTested(ConnectionTested {
            profile_id: cmd.gateway_profile_id,
            success: result.success,
            latency_ms: result.latency_ms,
            error_message: result.error_message.clone(),
            tested_at: Utc::now(),
        });

        Ok(TestConnectionResult {
            profile,
            success: result.success,
            latency_ms: result.latency_ms,
            event,
        })
    }

    async fn validate_credentials(&self, cmd: ValidateCredentials) -> Result<ValidateCredentialsResult, String> {
        let connector = self.registry.get(&cmd.connector_id)
            .map_err(|e| format!("Connector error: {}", e))?;

        let result = connector.validate_credentials(&cmd.config).await
            .map_err(|e| format!("Credential validation failed: {}", e))?;

        let event = GatewayEvent::CredentialsValidated(CredentialsValidated {
            connector_id: cmd.connector_id,
            valid: result.valid,
            merchant_name: result.merchant_name.clone(),
            validated_at: Utc::now(),
        });

        Ok(ValidateCredentialsResult {
            valid: result.valid,
            merchant_name: result.merchant_name,
            event,
        })
    }
}
