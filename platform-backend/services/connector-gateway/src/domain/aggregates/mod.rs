use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{
    FeeStructure, GatewayProfileStatus,
};
use shared_types::{CardScheme, CurrencyCode, Money};

#[derive(Debug, Clone)]
pub struct GatewayProfile {
    pub profile_id: Uuid,
    pub operator_id: Uuid,
    pub connector_id: String,
    pub merchant_acquirer_link_id: Uuid,
    pub status: GatewayProfileStatus,

    // Transaction Limits
    pub min_transaction_amount_minor: i64,
    pub max_transaction_amount_minor: i64,
    pub daily_volume_limit_minor: i64,
    pub monthly_volume_limit_minor: i64,
    pub max_refund_amount_minor: i64,

    // Fee Structure
    pub fixed_fee_minor: i64,
    pub percentage_fee_bps: i32,
    pub cross_border_fee_bps: i32,
    pub currency_conversion_fee_bps: i32,

    // Routing
    pub routing_priority: i32,
    pub base_url: String,
    pub enabled_card_schemes: Vec<CardScheme>,
    pub enabled_currencies: Vec<CurrencyCode>,
    pub enabled_countries: Vec<String>,

    // Rate Limiting
    pub rate_limit_per_second: u32,
    pub rate_limit_per_day: u32,

    // Monitoring
    pub success_rate_threshold: f64,
    pub latency_threshold_ms: u32,
    pub auto_disable_on_low_success: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GatewayProfile {
    pub fn new(
        operator_id: Uuid,
        connector_id: String,
        merchant_acquirer_link_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            profile_id: Uuid::now_v7(),
            operator_id,
            connector_id,
            merchant_acquirer_link_id,
            status: GatewayProfileStatus::Active,
            min_transaction_amount_minor: 100, // 1.00 AED
            max_transaction_amount_minor: 50_000_000, // 500,000 AED
            daily_volume_limit_minor: 5_000_000_000, // 50M AED
            monthly_volume_limit_minor: 50_000_000_000, // 500M AED
            max_refund_amount_minor: 50_000_000,
            fixed_fee_minor: 100, // 1.00 AED
            percentage_fee_bps: 250, // 2.50%
            cross_border_fee_bps: 0,
            currency_conversion_fee_bps: 0,
            routing_priority: 1,
            enabled_card_schemes: vec![CardScheme::Visa, CardScheme::Mastercard],
            enabled_currencies: vec![CurrencyCode::new("AED").unwrap()],
            enabled_countries: vec!["AE".to_string()],
            rate_limit_per_second: 100,
            rate_limit_per_day: 1_000_000,
            success_rate_threshold: 0.95,
            latency_threshold_ms: 5000,
            auto_disable_on_low_success: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate_transaction(
        &self,
        amount: &Money,
        card_scheme: &CardScheme,
        currency: &CurrencyCode,
    ) -> Result<(), GatewayError> {
        if amount.amount_minor_units < self.min_transaction_amount_minor {
            return Err(GatewayError::BelowMinimumAmount);
        }
        if amount.amount_minor_units > self.max_transaction_amount_minor {
            return Err(GatewayError::ExceedsMaximumAmount);
        }
        if !self.enabled_card_schemes.contains(card_scheme) {
            return Err(GatewayError::UnsupportedCardScheme);
        }
        if !self.enabled_currencies.contains(currency) {
            return Err(GatewayError::UnsupportedCurrency);
        }
        if self.status != GatewayProfileStatus::Active {
            return Err(GatewayError::GatewayDisabled);
        }
        Ok(())
    }

    pub fn calculate_fee(&self, amount: &Money, is_cross_border: bool, requires_fx: bool) -> Money {
        let fee_structure = FeeStructure {
            fixed_fee: Money {
                amount_minor_units: self.fixed_fee_minor,
                currency: amount.currency.clone(),
            },
            percentage_fee_bps: self.percentage_fee_bps,
            cross_border_fee_bps: self.cross_border_fee_bps,
            currency_conversion_fee_bps: self.currency_conversion_fee_bps,
        };
        fee_structure.calculate_fee(amount, is_cross_border, requires_fx)
    }
}

#[derive(Debug, Clone)]
pub enum GatewayError {
    BelowMinimumAmount,
    ExceedsMaximumAmount,
    UnsupportedCardScheme,
    UnsupportedCurrency,
    GatewayDisabled,
    DailyVolumeExceeded,
    MonthlyVolumeExceeded,
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BelowMinimumAmount => write!(f, "Amount below minimum"),
            Self::ExceedsMaximumAmount => write!(f, "Amount exceeds maximum"),
            Self::UnsupportedCardScheme => write!(f, "Unsupported card scheme"),
            Self::UnsupportedCurrency => write!(f, "Unsupported currency"),
            Self::GatewayDisabled => write!(f, "Gateway is disabled"),
            Self::DailyVolumeExceeded => write!(f, "Daily volume limit exceeded"),
            Self::MonthlyVolumeExceeded => write!(f, "Monthly volume limit exceeded"),
        }
    }
}

/// AcquirerConnector trait - the Anti-Corruption Layer interface
#[async_trait::async_trait]
pub trait AcquirerConnector: Send + Sync {
    fn connector_id(&self) -> &str;

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse, ConnectorError>;
    async fn capture(&self, req: CaptureRequest) -> Result<CaptureResponse, ConnectorError>;
    async fn void(&self, req: VoidRequest) -> Result<VoidResponse, ConnectorError>;
    async fn refund(&self, req: RefundRequest) -> Result<RefundResponse, ConnectorError>;
    async fn status_check(&self, req: StatusCheckRequest) -> Result<StatusCheckResponse, ConnectorError>;

    async fn poll_settlement(
        &self,
        req: PollSettlementRequest,
    ) -> Result<Vec<RawSettlementRecord>, ConnectorError>;

    fn verify_webhook_signature(
        &self,
        headers: &axum::http::HeaderMap,
        body: &[u8],
    ) -> Result<(), ConnectorError>;

    fn parse_webhook(&self, body: &[u8]) -> Result<ConnectorEvent, ConnectorError>;
}

use crate::domain::value_objects::*;
