use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::aggregates::GatewayProfile;
use crate::domain::value_objects::GatewayProfileStatus;
use crate::infrastructure::repository::GatewayProfileRepository;
use platform_error::PlatformError;
use platform_middleware::ssrf::{validate_url_async, SsrfCheckResult};
use shared_types::{CardScheme, CurrencyCode, Money};

#[async_trait]
pub trait GatewayService: Send + Sync {
    async fn create_profile(&self, cmd: CreateGatewayProfileCommand) -> Result<GatewayProfileResponse, PlatformError>;
    async fn update_profile(&self, cmd: UpdateGatewayProfileCommand) -> Result<(), PlatformError>;
    async fn get_profile(&self, profile_id: Uuid) -> Result<GatewayProfileResponse, PlatformError>;
    async fn list_profiles(&self, operator_id: Uuid) -> Result<Vec<GatewayProfileResponse>, PlatformError>;
    async fn validate_transaction(&self, cmd: ValidateTransactionCommand) -> Result<ValidationResult, PlatformError>;
    async fn select_gateway(&self, cmd: SelectGatewayCommand) -> Result<GatewaySelection, PlatformError>;
    /// Validate a webhook/callback URL per SRS SSRF-001/002.
    /// Checks: HTTPS-only, no private/reserved IPs, DNS rebinding protection.
    async fn validate_webhook_url(&self, url: &str) -> Result<WebhookUrlValidation, PlatformError>;
}

pub struct GatewayServiceImpl {
    repo: Box<dyn GatewayProfileRepository>,
}

impl GatewayServiceImpl {
    pub fn new(repo: Box<dyn GatewayProfileRepository>) -> Self {
        Self { repo }
    }
}

#[derive(Debug, Clone)]
pub struct CreateGatewayProfileCommand {
    pub operator_id: Uuid,
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub min_transaction_amount: Money,
    pub max_transaction_amount: Money,
    pub daily_volume_limit: Money,
    pub monthly_volume_limit: Money,
    pub fixed_fee: Money,
    pub percentage_fee_bps: i32,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<CurrencyCode>,
    pub routing_priority: i32,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct UpdateGatewayProfileCommand {
    pub profile_id: Uuid,
    pub status: Option<String>,
    pub min_transaction_amount: Option<Money>,
    pub max_transaction_amount: Option<Money>,
    pub fixed_fee: Option<Money>,
    pub percentage_fee_bps: Option<i32>,
    pub routing_priority: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ValidateTransactionCommand {
    pub profile_id: Uuid,
    pub amount: Money,
    pub card_scheme: CardScheme,
    pub currency: CurrencyCode,
}

#[derive(Debug, Clone)]
pub struct SelectGatewayCommand {
    pub operator_id: Uuid,
    pub amount: Money,
    pub card_scheme: CardScheme,
    pub currency: CurrencyCode,
}

#[derive(Debug, Clone)]
pub struct GatewayProfileResponse {
    pub profile_id: Uuid,
    pub connector_id: String,
    pub status: String,
    pub routing_priority: i32,
    pub base_url: String,
    pub min_amount: i64,
    pub max_amount: i64,
    pub daily_volume_limit: i64,
    pub fixed_fee: i64,
    pub percentage_fee_bps: i32,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub estimated_fee: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct GatewaySelection {
    pub profile_id: Uuid,
    pub connector_id: String,
    pub estimated_fee: Money,
}

#[derive(Debug, Clone)]
pub struct WebhookUrlValidation {
    pub valid: bool,
    pub url: String,
    pub error: Option<String>,
}

#[async_trait]
impl GatewayService for GatewayServiceImpl {
    async fn create_profile(&self, cmd: CreateGatewayProfileCommand) -> Result<GatewayProfileResponse, PlatformError> {
        // SRS SSRF-001/002: Validate acquirer API base URL before saving.
        // Uses validate_url_async to avoid blocking the Tokio runtime during DNS lookups.
        match validate_url_async(&cmd.base_url).await {
            SsrfCheckResult::Allowed => {}
            SsrfCheckResult::Blocked(reason) => {
                return Err(PlatformError::Validation(
                    platform_error::ValidationError::SsrfBlocked(reason)
                ));
            }
        }

        let mut profile = GatewayProfile::new(
            cmd.operator_id,
            cmd.connector_id,
            cmd.merchant_acquirer_link_id,
            cmd.base_url,
        );

        profile.min_transaction_amount_minor = cmd.min_transaction_amount.amount_minor_units;
        profile.max_transaction_amount_minor = cmd.max_transaction_amount.amount_minor_units;
        profile.daily_volume_limit_minor = cmd.daily_volume_limit.amount_minor_units;
        profile.monthly_volume_limit_minor = cmd.monthly_volume_limit.amount_minor_units;
        profile.fixed_fee_minor = cmd.fixed_fee.amount_minor_units;
        profile.percentage_fee_bps = cmd.percentage_fee_bps;
        profile.enabled_card_schemes = cmd.enabled_card_schemes;
        profile.enabled_currencies = cmd.enabled_currencies;
        profile.routing_priority = cmd.routing_priority;

        self.repo.save(&profile).await?;

        Ok(profile_to_response(&profile))
    }

    async fn update_profile(&self, cmd: UpdateGatewayProfileCommand) -> Result<(), PlatformError> {
        let mut profile = self.repo
            .load(cmd.profile_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "GatewayProfile".into(),
                id: cmd.profile_id,
            })?;

        if let Some(status) = cmd.status {
            profile.status = GatewayProfileStatus::from_str(&status)
                .map_err(|e| PlatformError::Validation(
                    platform_error::ValidationError::MissingField(e.to_string())
                ))?;
        }
        if let Some(min) = cmd.min_transaction_amount {
            profile.min_transaction_amount_minor = min.amount_minor_units;
        }
        if let Some(max) = cmd.max_transaction_amount {
            profile.max_transaction_amount_minor = max.amount_minor_units;
        }
        if let Some(fee) = cmd.fixed_fee {
            profile.fixed_fee_minor = fee.amount_minor_units;
        }
        if let Some(bps) = cmd.percentage_fee_bps {
            profile.percentage_fee_bps = bps;
        }
        if let Some(priority) = cmd.routing_priority {
            profile.routing_priority = priority;
        }

        profile.updated_at = chrono::Utc::now();
        self.repo.save(&profile).await?;

        Ok(())
    }

    async fn get_profile(&self, profile_id: Uuid) -> Result<GatewayProfileResponse, PlatformError> {
        let profile = self.repo
            .load(profile_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "GatewayProfile".into(),
                id: profile_id,
            })?;

        Ok(profile_to_response(&profile))
    }

    async fn list_profiles(&self, operator_id: Uuid) -> Result<Vec<GatewayProfileResponse>, PlatformError> {
        let profiles = self.repo.find_active_for_operator(operator_id).await?;
        Ok(profiles.iter().map(profile_to_response).collect())
    }

    async fn validate_transaction(&self, cmd: ValidateTransactionCommand) -> Result<ValidationResult, PlatformError> {
        let profile = self.repo
            .load(cmd.profile_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "GatewayProfile".into(),
                id: cmd.profile_id,
            })?;

        match profile.validate_transaction(&cmd.amount, &cmd.card_scheme, &cmd.currency) {
            Ok(()) => {
                let fee = profile.calculate_fee(&cmd.amount, false, false);
                Ok(ValidationResult {
                    valid: true,
                    error: None,
                    estimated_fee: Some(fee),
                })
            }
            Err(e) => Ok(ValidationResult {
                valid: false,
                error: Some(e.to_string()),
                estimated_fee: None,
            }),
        }
    }

    async fn select_gateway(&self, cmd: SelectGatewayCommand) -> Result<GatewaySelection, PlatformError> {
        let profiles = self.repo.find_active_for_operator(cmd.operator_id).await?;

        // Filter eligible profiles
        let eligible: Vec<&GatewayProfile> = profiles
            .iter()
            .filter(|p| p.validate_transaction(&cmd.amount, &cmd.card_scheme, &cmd.currency).is_ok())
            .collect();

        if eligible.is_empty() {
            return Err(PlatformError::Conflict(
                platform_error::ConflictError::IdempotencyKeyConflict,
            ));
        }

        // Select by priority (lowest number = highest priority)
        let selected = eligible
            .iter()
            .min_by_key(|p| p.routing_priority)
            .unwrap();

        let fee = selected.calculate_fee(&cmd.amount, false, false);

        Ok(GatewaySelection {
            profile_id: selected.profile_id,
            connector_id: selected.connector_id.clone(),
            estimated_fee: fee,
        })
    }

    /// SRS SSRF-001/002: Validate a webhook/callback URL using async DNS resolution.
    /// Uses validate_url_async to avoid blocking the Tokio runtime during DNS lookups.
    async fn validate_webhook_url(&self, url: &str) -> Result<WebhookUrlValidation, PlatformError> {
        match validate_url_async(url).await {
            SsrfCheckResult::Allowed => Ok(WebhookUrlValidation {
                valid: true,
                url: url.to_string(),
                error: None,
            }),
            SsrfCheckResult::Blocked(reason) => Ok(WebhookUrlValidation {
                valid: false,
                url: url.to_string(),
                error: Some(reason),
            }),
        }
    }
}

fn profile_to_response(p: &GatewayProfile) -> GatewayProfileResponse {
    GatewayProfileResponse {
        profile_id: p.profile_id,
        connector_id: p.connector_id.clone(),
        status: p.status.as_str().to_string(),
        routing_priority: p.routing_priority,
        base_url: p.base_url.clone(),
        min_amount: p.min_transaction_amount_minor,
        max_amount: p.max_transaction_amount_minor,
        daily_volume_limit: p.daily_volume_limit_minor,
        fixed_fee: p.fixed_fee_minor,
        percentage_fee_bps: p.percentage_fee_bps,
    }
}
