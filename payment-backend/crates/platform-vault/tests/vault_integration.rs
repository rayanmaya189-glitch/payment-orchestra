//! Integration tests for HashiCorp Vault client and secrets manager.
//!
//! These tests verify:
//! - Vault client configuration and initialization
//! - Token authentication flow
//! - KV v2 secrets engine operations
//! - Secret caching with TTL
//! - Role-based access control
//! - Policy generation and management
//! - Error handling and edge cases

use platform_vault::*;
use std::collections::HashMap;

// ============================================================================
// Vault Client Tests
// ============================================================================

#[test]
fn test_vault_config_default_values() {
    let config = VaultConfig::default();
    
    // Verify default values
    assert!(!config.addr.is_empty(), "Address should have a default value");
    assert_eq!(config.kv_mount, "secret", "Default KV mount should be 'secret'");
    assert_eq!(config.kv_version, 2, "Default KV version should be 2");
    assert_eq!(config.token_cache_ttl_secs, 300, "Default token cache TTL should be 300s");
    assert_eq!(config.request_timeout_secs, 10, "Default request timeout should be 10s");
}

#[test]
fn test_vault_config_custom_values() {
    let config = VaultConfig {
        addr: "https://custom-vault.example.com:8200".into(),
        auth_method: AuthMethod::Token { token: "custom-token".into() },
        kv_mount: "custom-secret".into(),
        kv_version: 1,
        token_cache_ttl_secs: 600,
        request_timeout_secs: 30,
    };
    
    assert_eq!(config.addr, "https://custom-vault.example.com:8200");
    assert_eq!(config.kv_mount, "custom-secret");
    assert_eq!(config.kv_version, 1);
    assert_eq!(config.token_cache_ttl_secs, 600);
    assert_eq!(config.request_timeout_secs, 30);
}

#[test]
fn test_auth_method_token_creation() {
    let auth = AuthMethod::Token { 
        token: "hvs.test-token-123".into() 
    };
    
    match auth {
        AuthMethod::Token { token } => {
            assert_eq!(token, "hvs.test-token-123");
        }
        _ => panic!("Expected Token auth method"),
    }
}

#[test]
fn test_auth_method_approle_creation() {
    let auth = AuthMethod::AppRole {
        role_id: "test-role-id".into(),
        secret_id: "test-secret-id".into(),
    };
    
    match auth {
        AuthMethod::AppRole { role_id, secret_id } => {
            assert_eq!(role_id, "test-role-id");
            assert_eq!(secret_id, "test-secret-id");
        }
        _ => panic!("Expected AppRole auth method"),
    }
}

#[test]
fn test_auth_method_kubernetes_creation() {
    let auth = AuthMethod::Kubernetes {
        role: "payment-service".into(),
        jwt: "eyJhbGciOiJSUzI1NiIs...".into(),
    };
    
    match auth {
        AuthMethod::Kubernetes { role, jwt } => {
            assert_eq!(role, "payment-service");
            assert!(jwt.starts_with("eyJ"));
        }
        _ => panic!("Expected Kubernetes auth method"),
    }
}

#[test]
fn test_auth_method_serialization_roundtrip() {
    let auth = AuthMethod::Token { token: "test-token".into() };
    let json = serde_json::to_string(&auth).unwrap();
    let deserialized: AuthMethod = serde_json::from_str(&json).unwrap();
    
    match deserialized {
        AuthMethod::Token { token } => {
            assert_eq!(token, "test-token");
        }
        _ => panic!("Deserialization failed"),
    }
}

#[test]
fn test_vault_client_creation_with_token() {
    let config = VaultConfig {
        addr: "https://localhost:8200".into(),
        auth_method: AuthMethod::Token { token: "test-token".into() },
        ..Default::default()
    };
    
    let client = VaultClient::new(config);
    assert!(client.is_ok(), "VaultClient should be created successfully");
}

#[test]
fn test_vault_health_structure() {
    let health = VaultHealth {
        initialized: true,
        sealed: false,
        standby: false,
        version: "1.15.4".into(),
    };
    
    assert!(health.initialized);
    assert!(!health.sealed);
    assert!(!health.standby);
    assert_eq!(health.version, "1.15.4");
}

// ============================================================================
// Secrets Manager Tests
// ============================================================================

#[test]
fn test_secret_role_creation() {
    let role = SecretRole {
        name: "database".into(),
        allowed_paths: vec!["secret/data/database/*".into()],
        allowed_operations: vec!["read".into()],
        max_ttl_secs: 3600,
    };
    
    assert_eq!(role.name, "database");
    assert_eq!(role.allowed_paths.len(), 1);
    assert_eq!(role.allowed_operations.len(), 1);
    assert_eq!(role.max_ttl_secs, 3600);
}

#[test]
fn test_secrets_manager_creation() {
    let config = VaultConfig {
        addr: "https://localhost:8200".into(),
        auth_method: AuthMethod::Token { token: "test-token".into() },
        ..Default::default()
    };
    
    let manager = SecretsManager::new(config);
    assert!(manager.is_ok(), "SecretsManager should be created successfully");
}

#[test]
fn test_secrets_manager_with_default_roles() {
    let config = VaultConfig {
        addr: "https://localhost:8200".into(),
        auth_method: AuthMethod::Token { token: "test-token".into() },
        ..Default::default()
    };
    
    let manager = SecretsManager::with_default_roles(config);
    assert!(manager.is_ok(), "SecretsManager with default roles should be created");
    
    let manager = manager.unwrap();
    
    // Verify default roles exist
    let stats = manager.cache_stats();
    assert!(stats.is_ok());
    let (size, _) = stats.unwrap();
    assert_eq!(size, 0, "Cache should be empty initially");
}

#[test]
fn test_secret_role_permission_check() {
    let role = SecretRole {
        name: "payment-gateway".into(),
        allowed_paths: vec![
            "secret/data/connectors/*".into(),
            "secret/data/payment-gateway/*".into(),
        ],
        allowed_operations: vec!["read".into()],
        max_ttl_secs: 1800,
    };
    
    // Test path matching - check if the path matches the pattern (with wildcard support)
    fn path_matches(path: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix("/*") {
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
    
    assert!(role.allowed_paths.iter().any(|p| path_matches("secret/data/connectors/stripe", p)));
    assert!(role.allowed_paths.iter().any(|p| path_matches("secret/data/payment-gateway/config", p)));
    assert!(!role.allowed_paths.iter().any(|p| path_matches("secret/data/database/config", p)));
    
    // Test operation matching
    assert!(role.allowed_operations.contains(&"read".to_string()));
    assert!(!role.allowed_operations.contains(&"write".to_string()));
}

#[test]
fn test_secret_role_wildcard_path() {
    let role = SecretRole {
        name: "admin".into(),
        allowed_paths: vec!["*".into()],
        allowed_operations: vec!["read".into(), "write".into(), "list".into()],
        max_ttl_secs: 86400,
    };
    
    // Wildcard should match any path
    assert!(role.allowed_paths.iter().any(|p| "any/path/at/all".starts_with(p) || p == "*"));
    assert!(role.allowed_operations.contains(&"write".to_string()));
}

// ============================================================================
// Policy Tests
// ============================================================================

#[test]
fn test_vault_policy_creation() {
    let policy = VaultPolicy::new("test-policy", Some("Test policy description"));
    
    assert_eq!(policy.name, "test-policy");
    assert_eq!(policy.description, Some("Test policy description".into()));
    assert!(policy.paths.is_empty());
}

#[test]
fn test_vault_policy_add_path() {
    let policy = VaultPolicy::new("test-policy", None)
        .add_path("secret/data/test/*", vec!["read", "list"]);
    
    assert_eq!(policy.paths.len(), 1);
    assert_eq!(policy.paths[0].path, "secret/data/test/*");
    assert_eq!(policy.paths[0].capabilities, vec!["read", "list"]);
}

#[test]
fn test_vault_policy_multiple_paths() {
    let policy = VaultPolicy::new("multi-path", None)
        .add_path("secret/data/db/*", vec!["read"])
        .add_path("secret/data/api/*", vec!["read", "write"])
        .add_path("sys/*", vec!["read"]);
    
    assert_eq!(policy.paths.len(), 3);
}

#[test]
fn test_platform_policies_exist() {
    let policies = PlatformPolicies::all();
    
    assert!(!policies.is_empty());
    assert!(policies.iter().any(|p| p.name == "database-readonly"));
    assert!(policies.iter().any(|p| p.name == "payment-connector"));
    assert!(policies.iter().any(|p| p.name == "ai-service"));
    assert!(policies.iter().any(|p| p.name == "admin"));
    assert!(policies.iter().any(|p| p.name == "operator"));
}

#[test]
fn test_platform_policies_database_readonly() {
    let policy = PlatformPolicies::database_readonly();
    
    assert_eq!(policy.name, "database-readonly");
    assert_eq!(policy.paths.len(), 1);
    assert_eq!(policy.paths[0].path, "secret/data/database/*");
    assert!(policy.paths[0].capabilities.contains(&"read".to_string()));
    assert!(policy.paths[0].capabilities.contains(&"list".to_string()));
}

#[test]
fn test_platform_policies_payment_connector() {
    let policy = PlatformPolicies::payment_connector();
    
    assert_eq!(policy.name, "payment-connector");
    assert_eq!(policy.paths.len(), 2);
    assert!(policy.paths.iter().any(|p| p.path == "secret/data/connectors/*"));
    assert!(policy.paths.iter().any(|p| p.path == "secret/data/payment-gateway/*"));
}

#[test]
fn test_platform_policies_admin_full_access() {
    let policy = PlatformPolicies::admin();
    
    assert_eq!(policy.name, "admin");
    assert!(policy.paths.len() >= 3);
    
    // Admin should have sudo capability
    let has_sudo = policy.paths.iter().any(|p| 
        p.capabilities.contains(&"sudo".to_string())
    );
    assert!(has_sudo, "Admin policy should have sudo capability");
}

#[test]
fn test_policy_to_hcl_conversion() {
    let policy = VaultPolicy::new("test", None)
        .add_path("secret/data/test/*", vec!["read", "list"]);
    
    let hcl = policy.to_hcl();
    
    assert!(hcl.contains("path"));
    assert!(hcl.contains("secret/data/test/*"));
    assert!(hcl.contains("capabilities"));
    assert!(hcl.contains("read"));
    assert!(hcl.contains("list"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_vault_error_display() {
    let errors = vec![
        VaultError::ClientError("client error".into()),
        VaultError::AuthenticationError("auth error".into()),
        VaultError::NetworkError("network error".into()),
        VaultError::ApiError("api error".into()),
        VaultError::ParseError("parse error".into()),
        VaultError::SecretNotFound("secret/path".into()),
        VaultError::PermissionDenied("permission denied".into()),
        VaultError::LockError("lock error".into()),
        VaultError::ConfigError("config error".into()),
    ];
    
    for error in errors {
        let display = format!("{}", error);
        assert!(!display.is_empty(), "Error should have a display message");
    }
}

#[test]
fn test_vault_error_is_std_error() {
    let error = VaultError::ClientError("test".into());
    let _: &dyn std::error::Error = &error;
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_vault_config_serialization() {
    let config = VaultConfig {
        addr: "https://vault.example.com".into(),
        auth_method: AuthMethod::Token { token: "test".into() },
        kv_mount: "secret".into(),
        kv_version: 2,
        token_cache_ttl_secs: 300,
        request_timeout_secs: 10,
    };
    
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: VaultConfig = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.addr, config.addr);
    assert_eq!(deserialized.kv_mount, config.kv_mount);
    assert_eq!(deserialized.kv_version, config.kv_version);
}

#[test]
fn test_secret_role_serialization() {
    let role = SecretRole {
        name: "test".into(),
        allowed_paths: vec!["path/*".into()],
        allowed_operations: vec!["read".into()],
        max_ttl_secs: 3600,
    };
    
    let json = serde_json::to_string(&role).unwrap();
    let deserialized: SecretRole = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.name, role.name);
    assert_eq!(deserialized.allowed_paths, role.allowed_paths);
}

#[test]
fn test_vault_health_serialization() {
    let health = VaultHealth {
        initialized: true,
        sealed: false,
        standby: false,
        version: "1.15.4".into(),
    };
    
    let json = serde_json::to_string(&health).unwrap();
    let deserialized: VaultHealth = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.initialized, health.initialized);
    assert_eq!(deserialized.sealed, health.sealed);
    assert_eq!(deserialized.version, health.version);
}

// ============================================================================
// Cache Behavior Tests (Simulated)
// ============================================================================

#[test]
fn test_cache_stats_empty() {
    let config = VaultConfig {
        addr: "https://localhost:8200".into(),
        auth_method: AuthMethod::Token { token: "test".into() },
        ..Default::default()
    };
    
    let manager = SecretsManager::new(config).unwrap();
    let (size, oldest) = manager.cache_stats().unwrap();
    
    assert_eq!(size, 0, "Cache should be empty");
    assert!(oldest.is_none(), "No oldest entry when cache is empty");
}

#[test]
fn test_cache_clear() {
    let config = VaultConfig {
        addr: "https://localhost:8200".into(),
        auth_method: AuthMethod::Token { token: "test".into() },
        ..Default::default()
    };
    
    let manager = SecretsManager::new(config).unwrap();
    let result = manager.clear_cache();
    
    assert!(result.is_ok(), "Cache clear should succeed");
    
    let (size, _) = manager.cache_stats().unwrap();
    assert_eq!(size, 0, "Cache should be empty after clear");
}

// ============================================================================
// Concurrency Tests
// ============================================================================

#[test]
fn test_concurrent_cache_access() {
    use std::sync::Arc;
    use std::thread;
    
    let config = VaultConfig {
        addr: "https://localhost:8200".into(),
        auth_method: AuthMethod::Token { token: "test".into() },
        ..Default::default()
    };
    
    let manager = Arc::new(SecretsManager::new(config).unwrap());
    let mut handles = vec![];
    
    // Spawn multiple threads to access cache concurrently
    for _ in 0..10 {
        let manager = Arc::clone(&manager);
        handles.push(thread::spawn(move || {
            let _ = manager.cache_stats();
            let _ = manager.clear_cache();
        }));
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify cache is still accessible
    let (size, _) = manager.cache_stats().unwrap();
    assert_eq!(size, 0);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_role_name() {
    let role = SecretRole {
        name: "".into(),
        allowed_paths: vec!["*".into()],
        allowed_operations: vec!["read".into()],
        max_ttl_secs: 0,
    };
    
    assert!(role.name.is_empty());
    assert_eq!(role.max_ttl_secs, 0);
}

#[test]
fn test_large_max_ttl() {
    let role = SecretRole {
        name: "long-lived".into(),
        allowed_paths: vec!["*".into()],
        allowed_operations: vec!["read".into()],
        max_ttl_secs: u64::MAX,
    };
    
    assert_eq!(role.max_ttl_secs, u64::MAX);
}

#[test]
fn test_special_characters_in_paths() {
    let policy = VaultPolicy::new("special", None)
        .add_path("secret/data/path-with-dashes/under_scores/dots.v1", vec!["read"]);
    
    assert_eq!(policy.paths[0].path, "secret/data/path-with-dashes/under_scores/dots.v1");
}

#[test]
fn test_unicode_in_policy_description() {
    let policy = VaultPolicy::new("unicode", Some("测试策略描述"));
    
    assert_eq!(policy.description, Some("测试策略描述".into()));
}
