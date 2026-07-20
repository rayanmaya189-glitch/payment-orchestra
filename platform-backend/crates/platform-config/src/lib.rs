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
    /// JWT signing secret — for HS256 (test/development only).
    /// Production MUST use RSA keys via jwt_private_key_pem / jwt_public_key_pem.
    pub jwt_secret: String,
    /// RSA private key PEM for JWT signing (SRS AUTH-017: RS256).
    /// Loaded from PLATFORM__AUTH__JWT_PRIVATE_KEY_PEM env var (multiline).
    /// When set, overrides jwt_secret for signing.
    pub jwt_private_key_pem: Option<String>,
    /// RSA public key PEM for JWT verification (SRS AUTH-017: RS256).
    /// Loaded from PLATFORM__AUTH__JWT_PUBLIC_KEY_PEM env var (multiline).
    /// When set, overrides jwt_secret for verification.
    pub jwt_public_key_pem: Option<String>,
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
                jwt_private_key_pem: None,
                jwt_public_key_pem: None,
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

// ==================== Feature Flag Management (SRS Part 19 §5) ====================

/// Feature flag targeting rules.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum FlagTargeting {
    /// Feature enabled for all users.
    Global,
    /// Feature enabled for a percentage of users (0-100).
    Percentage(u32),
    /// Feature enabled for specific segments.
    Segment(Vec<String>),
}

/// Feature flag definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlag {
    pub flag_key: String,
    pub enabled: bool,
    pub targeting: FlagTargeting,
    pub kill_switch: bool,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

impl FeatureFlag {
    pub fn new(flag_key: String, enabled: bool, description: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            flag_key,
            enabled,
            targeting: FlagTargeting::Global,
            kill_switch: false,
            description,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Check if the feature is enabled for a given context.
    pub fn is_enabled(&self, user_id: Option<&str>, segment: Option<&str>) -> bool {
        if self.kill_switch {
            return false;
        }
        if !self.enabled {
            return false;
        }
        match &self.targeting {
            FlagTargeting::Global => true,
            FlagTargeting::Percentage(pct) => {
                // Deterministic percentage based on user_id hash
                if let Some(uid) = user_id {
                    let hash = self.simple_hash(uid);
                    (hash % 100) < *pct
                } else {
                    false
                }
            }
            FlagTargeting::Segment(segments) => {
                if let Some(seg) = segment {
                    segments.contains(&seg.to_string())
                } else {
                    false
                }
            }
        }
    }

    /// Simple deterministic hash for percentage rollouts.
    fn simple_hash(&self, input: &str) -> u32 {
        let mut hash: u32 = 0;
        for byte in input.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        hash
    }
}

/// Feature flag store — in-memory with thread-safe access.
pub struct FeatureFlagStore {
    flags: std::sync::RwLock<Vec<FeatureFlag>>,
}

impl FeatureFlagStore {
    pub fn new() -> Self {
        Self {
            flags: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Add or update a feature flag.
    pub fn set_flag(&self, flag: FeatureFlag) {
        let mut flags = self.flags.write().unwrap();
        if let Some(existing) = flags.iter_mut().find(|f| f.flag_key == flag.flag_key) {
            *existing = flag;
        } else {
            flags.push(flag);
        }
    }

    /// Check if a feature is enabled.
    pub fn is_enabled(&self, flag_key: &str, user_id: Option<&str>, segment: Option<&str>) -> bool {
        let flags = self.flags.read().unwrap();
        flags.iter()
            .find(|f| f.flag_key == flag_key)
            .map(|f| f.is_enabled(user_id, segment))
            .unwrap_or(false) // Unknown flags default to disabled
    }

    /// Get all flags.
    pub fn list_flags(&self) -> Vec<FeatureFlag> {
        self.flags.read().unwrap().clone()
    }

    /// Remove a flag.
    pub fn remove_flag(&self, flag_key: &str) -> bool {
        let mut flags = self.flags.write().unwrap();
        let len_before = flags.len();
        flags.retain(|f| f.flag_key != flag_key);
        flags.len() < len_before
    }
}

impl Default for FeatureFlagStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flag_global() {
        let flag = FeatureFlag::new("test_feature".into(), true, "Test feature".into());
        assert!(flag.is_enabled(None, None));
    }

    #[test]
    fn test_feature_flag_disabled() {
        let flag = FeatureFlag::new("test_feature".into(), false, "Test feature".into());
        assert!(!flag.is_enabled(None, None));
    }

    #[test]
    fn test_feature_flag_kill_switch() {
        let mut flag = FeatureFlag::new("test_feature".into(), true, "Test feature".into());
        flag.kill_switch = true;
        assert!(!flag.is_enabled(None, None));
    }

    #[test]
    fn test_feature_flag_percentage() {
        let flag = FeatureFlag {
            flag_key: "test_feature".into(),
            enabled: true,
            targeting: FlagTargeting::Percentage(50),
            kill_switch: false,
            description: "Test".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        // Should be deterministic for same user
        let result1 = flag.is_enabled(Some("user_123"), None);
        let result2 = flag.is_enabled(Some("user_123"), None);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_feature_flag_segment() {
        let flag = FeatureFlag {
            flag_key: "test_feature".into(),
            enabled: true,
            targeting: FlagTargeting::Segment(vec!["beta".into(), "premium".into()]),
            kill_switch: false,
            description: "Test".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert!(flag.is_enabled(None, Some("beta")));
        assert!(flag.is_enabled(None, Some("premium")));
        assert!(!flag.is_enabled(None, Some("basic")));
    }

    #[test]
    fn test_feature_flag_store() {
        let store = FeatureFlagStore::new();
        store.set_flag(FeatureFlag::new("feature_a".into(), true, "Feature A".into()));
        store.set_flag(FeatureFlag::new("feature_b".into(), false, "Feature B".into()));

        assert!(store.is_enabled("feature_a", None, None));
        assert!(!store.is_enabled("feature_b", None, None));
        assert!(!store.is_enabled("unknown_feature", None, None)); // Unknown = disabled

        assert_eq!(store.list_flags().len(), 2);
        assert!(store.remove_flag("feature_a"));
        assert_eq!(store.list_flags().len(), 1);
    }

    #[test]
    fn test_feature_flag_store_update() {
        let store = FeatureFlagStore::new();
        store.set_flag(FeatureFlag::new("feature_a".into(), false, "Feature A".into()));
        assert!(!store.is_enabled("feature_a", None, None));

        // Update the flag
        store.set_flag(FeatureFlag::new("feature_a".into(), true, "Feature A updated".into()));
        assert!(store.is_enabled("feature_a", None, None));
        assert_eq!(store.list_flags().len(), 1); // Still only 1 flag
    }
}
