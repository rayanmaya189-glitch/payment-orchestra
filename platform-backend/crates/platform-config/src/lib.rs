use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub auth: AuthConfig,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub health_port: u16,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
    pub min_connections: u32,
    pub max_lifetime_secs: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct NatsConfig {
    pub url: String,
}

/// Authentication configuration — JWT, API keys, MFA, lockout policy.
/// All secrets loaded from environment variables, NEVER hardcoded.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AuthConfig {
    /// JWT signing secret — loaded from PLATFORM__AUTH__JWT_SECRET env var.
    /// CRITICAL: Must be at least 256 bits (32 bytes) for HS256.
    pub jwt_secret: String,
    /// JWT access token lifetime in seconds (default: 900 = 15 minutes per SRS AUTH-002)
    pub jwt_access_token_ttl_secs: u64,
    /// JWT refresh token lifetime in seconds (default: 604800 = 7 days per SRS AUTH-002)
    pub jwt_refresh_token_ttl_secs: u64,
    /// Refresh token rotation window — if a refresh token is used after this
    /// many seconds since last rotation, invalidate all sessions (SRS SESS-SEC-003)
    pub refresh_token_rotation_window_secs: u64,
    /// Account lockout thresholds (SRS AUTH-007)
    pub lockout_attempts_15min: i32,
    pub lockout_attempts_1hr: i32,
    pub lockout_attempts_suspend: i32,
    /// Password minimum length (SRS AUTH-003)
    pub password_min_length: usize,
    /// API key default expiry in days (SRS AUTH-004)
    pub api_key_default_expiry_days: u32,
    /// API key maximum expiry in days (SRS AUTH-004)
    pub api_key_max_expiry_days: u32,
}

/// CORS configuration per SRS CORS-001
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CorsConfig {
    /// Comma-separated list of allowed origins
    pub allowed_origins: String,
    /// Max age for preflight cache in seconds (default: 86400)
    pub max_age_secs: u64,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Login attempts per IP per minute (SRS AUTH-009)
    pub login_per_ip_per_minute: u32,
    /// General API requests per second per principal
    pub api_per_principal_per_second: u32,
    /// Credential decryptions per principal per hour (SRS CRED-MON-002)
    pub credential_access_per_hour: u32,
}

impl AppConfig {
    pub fn from_env(service_name: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(
                Environment::with_prefix(&service_name.to_uppercase())
                    .separator("__")
                    .try_parsing(true),
            )
            .add_source(
                Environment::with_prefix("PLATFORM")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Production config: reads from env vars, panics if critical vars are missing.
    pub fn from_env_or_panic(service_name: &str) -> Self {
        Self::from_env(service_name).unwrap_or_else(|e| {
            panic!(
                "Failed to load config for {} from environment: {}. \
                 Set PLATFORM__AUTH__JWT_SECRET and other required env vars.",
                service_name, e
            )
        })
    }
}

/// Default config for tests — uses safe, deterministic defaults.
/// NEVER reads env vars (test-friendly, no side effects).
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                health_port: 8081,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/platform".to_string(),
                max_connections: 10,
                connection_timeout_secs: 5,
                min_connections: 1,
                max_lifetime_secs: 1800,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
            },
            nats: NatsConfig {
                url: "nats://localhost:4222".to_string(),
            },
            auth: AuthConfig {
                // Test-only default — production MUST use from_env_or_panic()
                jwt_secret: "test-only-insecure-secret-not-for-production-32bytes!!".to_string(),
                jwt_access_token_ttl_secs: 900,        // 15 minutes
                jwt_refresh_token_ttl_secs: 604800,     // 7 days
                refresh_token_rotation_window_secs: 300, // 5 minutes
                lockout_attempts_15min: 5,
                lockout_attempts_1hr: 10,
                lockout_attempts_suspend: 20,
                password_min_length: 12,
                api_key_default_expiry_days: 90,
                api_key_max_expiry_days: 365,
            },
            cors: CorsConfig {
                allowed_origins: "http://localhost:3000".to_string(),
                max_age_secs: 86400,
            },
            rate_limit: RateLimitConfig {
                login_per_ip_per_minute: 10,
                api_per_principal_per_second: 100,
                credential_access_per_hour: 10,
            },
        }
    }
}
