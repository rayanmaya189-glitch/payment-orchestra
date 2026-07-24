use uuid::Uuid;

/// Errors that can occur during connector operations.
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

/// Errors specific to gateway profile operations.
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
