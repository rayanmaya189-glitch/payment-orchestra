//! Rate limit configuration — reads from environment variables.
//!
//! Environment variables:
//! - `RATE_LIMIT_SERVICE_MAX` — max requests per service per window (default: 1000)
//! - `RATE_LIMIT_CLIENT_MAX` — max requests per client per window (default: 100)
//! - `RATE_LIMIT_WINDOW_SECS` — window size in seconds (default: 60)
//!
//! Per-service overrides (higher-priority services get stricter limits):
//! - `RATE_LIMIT_API_GATEWAY_MAX` — override for API gateway (default: 2000)
//! - `RATE_LIMIT_IAM_SERVICE_MAX` — override for IAM service (default: 500)
//! - `RATE_LIMIT_ORCHESTRATION_MAX` — override for orchestration (default: 1500)

/// Parsed rate limit configuration for a single service.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub service_max: u32,
    pub client_max: u32,
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            service_max: env_or("RATE_LIMIT_SERVICE_MAX", 1000),
            client_max: env_or("RATE_LIMIT_CLIENT_MAX", 100),
            window_secs: env_or_u64("RATE_LIMIT_WINDOW_SECS", 60),
        }
    }
}

impl RateLimitConfig {
    /// Get the config for a specific service, applying overrides if set.
    pub fn for_service(service_name: &str) -> Self {
        let base = Self::default();

        // Check for per-service override
        let override_key = format!(
            "RATE_LIMIT_{}_MAX",
            service_name.to_uppercase().replace('-', "_")
        );

        let service_max = std::env::var(&override_key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(base.service_max);

        Self {
            service_max,
            client_max: base.client_max,
            window_secs: base.window_secs,
        }
    }
}

fn env_or(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_or_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.service_max, 1000);
        assert_eq!(config.client_max, 100);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_for_service_no_override() {
        let config = RateLimitConfig::for_service("api-gateway");
        assert_eq!(config.service_max, 1000);
    }

    #[test]
    fn test_for_service_with_override() {
        unsafe {
            std::env::set_var("RATE_LIMIT_API_GATEWAY_MAX", "2000");
        }
        let config = RateLimitConfig::for_service("api-gateway");
        assert_eq!(config.service_max, 2000);
        unsafe {
            std::env::remove_var("RATE_LIMIT_API_GATEWAY_MAX");
        }
    }
}
