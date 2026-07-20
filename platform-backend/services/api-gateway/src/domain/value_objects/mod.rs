#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

/// API key status for authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

impl ApiKeyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            "expired" => Ok(Self::Expired),
            _ => Err("unknown API key status"),
        }
    }
}

/// Rate limit window for API key throttling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitWindow {
    pub window_seconds: u32,
    pub max_requests: u32,
}

impl Default for RateLimitWindow {
    fn default() -> Self {
        Self {
            window_seconds: 60,
            max_requests: 100,
        }
    }
}

/// Health check status for downstream services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub service_name: String,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub last_checked: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_status_values() {
        assert_eq!(ApiKeyStatus::Active.as_str(), "active");
        assert_eq!(ApiKeyStatus::Revoked.as_str(), "revoked");
        assert_eq!(ApiKeyStatus::Expired.as_str(), "expired");
    }

    #[test]
    fn test_api_key_status_from_str() {
        assert_eq!(ApiKeyStatus::from_str("active").unwrap(), ApiKeyStatus::Active);
        assert!(ApiKeyStatus::from_str("unknown").is_err());
    }

    #[test]
    fn test_rate_limit_default() {
        let rl = RateLimitWindow::default();
        assert_eq!(rl.window_seconds, 60);
        assert_eq!(rl.max_requests, 100);
    }

    #[test]
    fn test_health_status_values() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
    }
}
