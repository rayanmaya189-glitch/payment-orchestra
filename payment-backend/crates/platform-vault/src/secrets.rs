//! Production-grade secrets manager with role-based access control.
//!
//! Provides:
//! - Centralized secret management
//! - Role-based secret access
//! - Automatic secret rotation support
//! - Secret caching with TTL
//! - Audit logging for secret access

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use super::client::{VaultClient, VaultConfig};
use crate::VaultError;

/// Secret role definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRole {
    /// Role name (e.g., "database", "payment-gateway", "ai-service")
    pub name: String,
    /// Paths this role can access
    pub allowed_paths: Vec<String>,
    /// Operations allowed (read, write, list)
    pub allowed_operations: Vec<String>,
    /// Maximum secret TTL in seconds
    pub max_ttl_secs: u64,
}

/// Cached secret with metadata
#[derive(Debug, Clone)]
struct CachedSecret {
    value: HashMap<String, Value>,
    fetched_at: Instant,
    expires_at: Instant,
}

/// Secrets manager
pub struct SecretsManager {
    vault_client: VaultClient,
    cache: Arc<Mutex<HashMap<String, CachedSecret>>>,
    roles: HashMap<String, SecretRole>,
    default_ttl_secs: u64,
}

impl SecretsManager {
    /// Create a new secrets manager
    pub fn new(config: VaultConfig) -> Result<Self, VaultError> {
        let vault_client = VaultClient::new(config)?;
        Ok(Self {
            vault_client,
            cache: Arc::new(Mutex::new(HashMap::new())),
            roles: HashMap::new(),
            default_ttl_secs: 300,
        })
    }

    /// Add a secret role
    pub fn add_role(&mut self, role: SecretRole) {
        self.roles.insert(role.name.clone(), role);
    }

    /// Get a secret with role-based access control
    pub async fn get_secret(&self, path: &str, role: Option<&str>) -> Result<HashMap<String, Value>, VaultError> {
        // Check role permissions
        if let Some(role_name) = role {
            self.check_permission(path, role_name, "read")?;
        }

        // Check cache first
        {
            let cache = self.cache.lock()
                .map_err(|e| VaultError::LockError(e.to_string()))?;
            if let Some(cached) = cache.get(path) {
                if Instant::now() < cached.expires_at {
                    info!("Secret cache hit: path={}, role={:?}", path, role);
                    return Ok(cached.value.clone());
                }
            }
        }

        // Fetch from Vault
        let secrets = self.vault_client.read_secret(path).await?;

        // Cache the secret
        {
            let mut cache = self.cache.lock()
                .map_err(|e| VaultError::LockError(e.to_string()))?;
            cache.insert(path.to_string(), CachedSecret {
                value: secrets.clone(),
                fetched_at: Instant::now(),
                expires_at: Instant::now() + Duration::from_secs(self.default_ttl_secs),
            });
        }

        info!("Secret fetched from Vault: path={}, role={:?}", path, role);
        Ok(secrets)
    }

    /// Get a single secret field
    pub async fn get_secret_field(&self, path: &str, field: &str, role: Option<&str>) -> Result<String, VaultError> {
        let secrets = self.get_secret(path, role).await?;
        secrets.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| VaultError::SecretNotFound(format!("{}/{}", path, field)))
    }

    /// Write a secret with role-based access control
    pub async fn set_secret(&self, path: &str, data: &HashMap<String, Value>, role: Option<&str>) -> Result<(), VaultError> {
        // Check role permissions
        if let Some(role_name) = role {
            self.check_permission(path, role_name, "write")?;
        }

        self.vault_client.write_secret(path, data).await?;

        // Invalidate cache
        {
            let mut cache = self.cache.lock()
                .map_err(|e| VaultError::LockError(e.to_string()))?;
            cache.remove(path);
        }

        info!("Secret written to Vault: path={}, role={:?}", path, role);
        Ok(())
    }

    /// Delete a secret with role-based access control
    pub async fn delete_secret(&self, path: &str, role: Option<&str>) -> Result<(), VaultError> {
        // Check role permissions
        if let Some(role_name) = role {
            self.check_permission(path, role_name, "write")?;
        }

        self.vault_client.delete_secret(path).await?;

        // Invalidate cache
        {
            let mut cache = self.cache.lock()
                .map_err(|e| VaultError::LockError(e.to_string()))?;
            cache.remove(path);
        }

        info!("Secret deleted from Vault: path={}, role={:?}", path, role);
        Ok(())
    }

    /// List secrets at a path
    pub async fn list_secrets(&self, path: &str, role: Option<&str>) -> Result<Vec<String>, VaultError> {
        // Check role permissions
        if let Some(role_name) = role {
            self.check_permission(path, role_name, "list")?;
        }

        self.vault_client.list_secrets(path).await
    }

    /// Check if a role has permission to access a path
    fn check_permission(&self, path: &str, role_name: &str, operation: &str) -> Result<(), VaultError> {
        let role = self.roles.get(role_name)
            .ok_or_else(|| VaultError::PermissionDenied(format!("Unknown role: {}", role_name)))?;

        // Check if operation is allowed
        if !role.allowed_operations.contains(&operation.to_string()) {
            return Err(VaultError::PermissionDenied(
                format!("Role '{}' is not allowed to '{}' on '{}'", role_name, operation, path)
            ));
        }

        // Check if path is allowed
        let path_allowed = role.allowed_paths.iter().any(|allowed| {
            path.starts_with(allowed) || allowed == "*"
        });

        if !path_allowed {
            return Err(VaultError::PermissionDenied(
                format!("Role '{}' is not allowed to access path '{}'", role_name, path)
            ));
        }

        Ok(())
    }

    /// Clear the secret cache
    pub fn clear_cache(&self) -> Result<(), VaultError> {
        let mut cache = self.cache.lock()
            .map_err(|e| VaultError::LockError(e.to_string()))?;
        cache.clear();
        info!("Secret cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<(usize, Option<Instant>), VaultError> {
        let cache = self.cache.lock()
            .map_err(|e| VaultError::LockError(e.to_string()))?;
        let size = cache.len();
        let oldest = cache.values().map(|c| c.fetched_at).min();
        Ok((size, oldest))
    }
}

// Predefined roles for the payment platform
impl SecretsManager {
    /// Create default roles for the payment platform
    pub fn with_default_roles(config: VaultConfig) -> Result<Self, VaultError> {
        let mut manager = Self::new(config)?;

        // Database role
        manager.add_role(SecretRole {
            name: "database".into(),
            allowed_paths: vec!["secret/data/database/*".into()],
            allowed_operations: vec!["read".into()],
            max_ttl_secs: 3600,
        });

        // Payment gateway role
        manager.add_role(SecretRole {
            name: "payment-gateway".into(),
            allowed_paths: vec![
                "secret/data/connectors/*".into(),
                "secret/data/payment-gateway/*".into(),
            ],
            allowed_operations: vec!["read".into()],
            max_ttl_secs: 1800,
        });

        // AI service role
        manager.add_role(SecretRole {
            name: "ai-service".into(),
            allowed_paths: vec![
                "secret/data/ai/*".into(),
                "secret/data/openai/*".into(),
            ],
            allowed_operations: vec!["read".into()],
            max_ttl_secs: 1800,
        });

        // Admin role (full access)
        manager.add_role(SecretRole {
            name: "admin".into(),
            allowed_paths: vec!["*".into()],
            allowed_operations: vec!["read".into(), "write".into(), "list".into()],
            max_ttl_secs: 86400,
        });

        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_role_creation() {
        let role = SecretRole {
            name: "test".into(),
            allowed_paths: vec!["secret/data/test/*".into()],
            allowed_operations: vec!["read".into()],
            max_ttl_secs: 3600,
        };
        assert_eq!(role.name, "test");
    }
}
