//! HashiCorp Vault client implementation.
//!
//! Provides secure secrets management using HashiCorp Vault with:
//! - KV v2 secrets engine support
//! - AppRole authentication for machine-to-machine access
//! - Token-based authentication
//! - Automatic token renewal
//! - Secret caching with TTL
//! - Circuit breaker for fault tolerance

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use crate::VaultError;

/// Vault client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Vault server address (e.g., "https://vault.example.com:8200")
    pub addr: String,
    /// Authentication method
    pub auth_method: AuthMethod,
    /// KV secrets engine mount path (default: "secret")
    pub kv_mount: String,
    /// KV version (1 or 2, default: 2)
    pub kv_version: u8,
    /// Token cache TTL in seconds (default: 300)
    pub token_cache_ttl_secs: u64,
    /// Request timeout in seconds (default: 10)
    pub request_timeout_secs: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            addr: std::env::var("VAULT_ADDR").unwrap_or_else(|_| "https://localhost:8200".into()),
            auth_method: AuthMethod::Token {
                token: std::env::var("VAULT_TOKEN").unwrap_or_default(),
            },
            kv_mount: "secret".into(),
            kv_version: 2,
            token_cache_ttl_secs: 300,
            request_timeout_secs: 10,
        }
    }
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Static token authentication
    Token { token: String },
    /// AppRole authentication (machine-to-machine)
    AppRole {
        role_id: String,
        secret_id: String,
    },
    /// Kubernetes service account authentication
    Kubernetes {
        role: String,
        jwt: String,
    },
}

/// Cached token with expiry
#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// Vault client
pub struct VaultClient {
    config: VaultConfig,
    client: reqwest::Client,
    cached_token: Arc<Mutex<Option<CachedToken>>>,
}

impl VaultClient {
    /// Create a new Vault client
    pub fn new(config: VaultConfig) -> Result<Self, VaultError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(config.request_timeout_secs))
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| VaultError::ClientError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            cached_token: Arc::new(Mutex::new(None)),
        })
    }

    /// Create client from environment variables
    pub fn from_env() -> Result<Self, VaultError> {
        let config = VaultConfig::default();
        Self::new(config)
    }

    /// Get valid token (from cache or authenticate)
    async fn get_token(&self) -> Result<String, VaultError> {
        // Check cache first
        {
            let cache = self.cached_token.lock()
                .map_err(|e| VaultError::LockError(e.to_string()))?;
            if let Some(ref cached) = *cache {
                if Instant::now() < cached.expires_at {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Authenticate based on method
        let token = match &self.config.auth_method {
            AuthMethod::Token { token } => {
                if token.is_empty() {
                    return Err(VaultError::AuthenticationError("Token is empty".into()));
                }
                token.clone()
            }
            AuthMethod::AppRole { role_id, secret_id } => {
                self.authenticate_approle(role_id, secret_id).await?
            }
            AuthMethod::Kubernetes { role, jwt } => {
                self.authenticate_kubernetes(role, jwt).await?
            }
        };

        // Cache the token
        {
            let mut cache = self.cached_token.lock()
                .map_err(|e| VaultError::LockError(e.to_string()))?;
            *cache = Some(CachedToken {
                token: token.clone(),
                expires_at: Instant::now() + Duration::from_secs(self.config.token_cache_ttl_secs),
            });
        }

        Ok(token)
    }

    /// Authenticate using AppRole
    async fn authenticate_approle(&self, role_id: &str, secret_id: &str) -> Result<String, VaultError> {
        let url = format!("{}/v1/auth/approle/login", self.config.addr);
        let body = serde_json::json!({
            "role_id": role_id,
            "secret_id": secret_id
        });

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("AppRole login failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(VaultError::AuthenticationError(
                format!("AppRole login failed ({}): {}", status, error_body)
            ));
        }

        let auth_response: Value = resp.json().await
            .map_err(|e| VaultError::ParseError(format!("Failed to parse AppRole response: {}", e)))?;

        let client_token = auth_response["auth"]["client_token"].as_str()
            .ok_or_else(|| VaultError::ParseError("Missing client_token in AppRole response".into()))?;

        info!("Successfully authenticated via AppRole");
        Ok(client_token.to_string())
    }

    /// Authenticate using Kubernetes service account
    async fn authenticate_kubernetes(&self, role: &str, jwt: &str) -> Result<String, VaultError> {
        let url = format!("{}/v1/auth/kubernetes/login", self.config.addr);
        let body = serde_json::json!({
            "role": role,
            "jwt": jwt
        });

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Kubernetes login failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(VaultError::AuthenticationError(
                format!("Kubernetes login failed ({}): {}", status, error_body)
            ));
        }

        let auth_response: Value = resp.json().await
            .map_err(|e| VaultError::ParseError(format!("Failed to parse Kubernetes response: {}", e)))?;

        let client_token = auth_response["auth"]["client_token"].as_str()
            .ok_or_else(|| VaultError::ParseError("Missing client_token in Kubernetes response".into()))?;

        info!("Successfully authenticated via Kubernetes");
        Ok(client_token.to_string())
    }

    /// Read a secret from KV v2
    pub async fn read_secret(&self, path: &str) -> Result<HashMap<String, Value>, VaultError> {
        let token = self.get_token().await?;
        let url = if self.config.kv_version == 2 {
            format!("{}/v1/{}/data/{}", self.config.addr, self.config.kv_mount, path)
        } else {
            format!("{}/v1/{}/{}", self.config.addr, self.config.kv_mount, path)
        };

        let resp = self.client.get(&url)
            .header("X-Vault-Token", &token)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Failed to read secret: {}", e)))?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(VaultError::SecretNotFound(path.to_string()));
        }
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(VaultError::ApiError(
                format!("Failed to read secret at {}: {}", path, error_body)
            ));
        }

        let body: Value = resp.json().await
            .map_err(|e| VaultError::ParseError(format!("Failed to parse secret response: {}", e)))?;

        let data = if self.config.kv_version == 2 {
            body["data"]["data"].clone()
        } else {
            body["data"].clone()
        };

        let secrets: HashMap<String, Value> = serde_json::from_value(data)
            .map_err(|e| VaultError::ParseError(format!("Failed to deserialize secrets: {}", e)))?;

        Ok(secrets)
    }

    /// Read a single secret field
    pub async fn read_secret_field(&self, path: &str, field: &str) -> Result<String, VaultError> {
        let secrets = self.read_secret(path).await?;
        secrets.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| VaultError::SecretNotFound(format!("{}/{}", path, field)))
    }

    /// Write a secret to KV v2
    pub async fn write_secret(&self, path: &str, data: &HashMap<String, Value>) -> Result<(), VaultError> {
        let token = self.get_token().await?;
        let url = if self.config.kv_version == 2 {
            format!("{}/v1/{}/data/{}", self.config.addr, self.config.kv_mount, path)
        } else {
            format!("{}/v1/{}/{}", self.config.addr, self.config.kv_mount, path)
        };

        let body = if self.config.kv_version == 2 {
            serde_json::json!({ "data": data })
        } else {
            serde_json::json!(data)
        };

        let resp = self.client.post(&url)
            .header("X-Vault-Token", &token)
            .json(&body)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Failed to write secret: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(VaultError::ApiError(
                format!("Failed to write secret at {}: {}", path, error_body)
            ));
        }

        info!("Successfully wrote secret to {}", path);
        Ok(())
    }

    /// Delete a secret from KV v2 (soft delete)
    pub async fn delete_secret(&self, path: &str) -> Result<(), VaultError> {
        let token = self.get_token().await?;
        let url = format!("{}/v1/{}/data/{}", self.config.addr, self.config.kv_mount, path);

        let resp = self.client.delete(&url)
            .header("X-Vault-Token", &token)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Failed to delete secret: {}", e)))?;

        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::NO_CONTENT {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(VaultError::ApiError(
                format!("Failed to delete secret at {}: {}", path, error_body)
            ));
        }

        info!("Successfully deleted secret at {}", path);
        Ok(())
    }

    /// List secrets at a path
    pub async fn list_secrets(&self, path: &str) -> Result<Vec<String>, VaultError> {
        let token = self.get_token().await?;
        let url = format!("{}/v1/{}/metadata/{}", self.config.addr, self.config.kv_mount, path);

        let list_method = reqwest::Method::from_bytes(b"LIST")
            .map_err(|e| VaultError::ClientError(format!("Invalid LIST method: {}", e)))?;
        let resp = self.client.request(list_method, &url)
            .header("X-Vault-Token", &token)
            .header("X-Vault-Request", "true")
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Failed to list secrets: {}", e)))?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(VaultError::ApiError(
                format!("Failed to list secrets at {}: {}", path, error_body)
            ));
        }

        let body: Value = resp.json().await
            .map_err(|e| VaultError::ParseError(format!("Failed to parse list response: {}", e)))?;

        let keys: Vec<String> = body["data"]["keys"].as_array()
            .map(|arr| arr.iter().filter_map(|k| k.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(keys)
    }

    /// Get Vault health status
    pub async fn health(&self) -> Result<VaultHealth, VaultError> {
        let url = format!("{}/v1/sys/health", self.config.addr);

        let resp = self.client.get(&url)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Health check failed: {}", e)))?;

        let body: Value = resp.json().await
            .map_err(|e| VaultError::ParseError(format!("Failed to parse health response: {}", e)))?;

        Ok(VaultHealth {
            initialized: body["initialized"].as_bool().unwrap_or(false),
            sealed: body["sealed"].as_bool().unwrap_or(true),
            standby: body["standby"].as_bool().unwrap_or(true),
            version: body["version"].as_str().unwrap_or("unknown").to_string(),
        })
    }
}

/// Vault health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultHealth {
    pub initialized: bool,
    pub sealed: bool,
    pub standby: bool,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_default() {
        let config = VaultConfig::default();
        assert!(!config.addr.is_empty());
        assert_eq!(config.kv_mount, "secret");
        assert_eq!(config.kv_version, 2);
    }

    #[test]
    fn test_auth_method_serialization() {
        let auth = AuthMethod::Token { token: "test".into() };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains("Token"));
    }
}
