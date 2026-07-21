//! # Platform Vault - HashiCorp Vault KV Store Integration
//!
//! Provides key management via HashiCorp Vault KV Store for:
//! - Data Encryption Keys (DEK) for PII field encryption
//! - JWT signing and verification keys
//! - API keys and connector credentials
//!
//! ## Features
//! - `HttpVaultClient`: Production client using Vault HTTP API
//! - `InMemoryVaultClient`: Development/testing fallback
//! - Support for KV Store v1 and v2
//!
//! ## Usage
//!
//! ```rust,no_run
//! use platform_vault::{VaultConfig, HttpVaultClient, VaultClient};
//!
//! # async fn example() -> Result<(), platform_error::PlatformError> {
//! let config = VaultConfig {
//!     url: "https://vault.example.com:8200".to_string(),
//!     token: std::env::var("VAULT_TOKEN").unwrap(),
//!     mount_point: "secret".to_string(),
//!     kv_version: platform_vault::SecretEngine::KvV2,
//! };
//!
//! let client = HttpVaultClient::new(config)?;
//!
//! // Retrieve a secret
//! let jwt_key = client.get_secret("data/auth/jwt-signing-key").await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use platform_error::PlatformError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, error, info, instrument};

/// Secret engine version support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecretEngine {
    #[serde(rename = "kv-v1")]
    KvV1,
    #[serde(rename = "kv-v2")]
    KvV2,
}

impl Default for SecretEngine {
    fn default() -> Self {
        Self::KvV2
    }
}

/// Vault client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Vault server URL (e.g., "https://vault.example.com:8200")
    pub url: String,
    /// Authentication token
    pub token: String,
    /// Secret engine mount point (default: "secret")
    #[serde(default = "default_mount_point")]
    pub mount_point: String,
    /// KV Store version
    #[serde(default)]
    pub kv_version: SecretEngine,
}

fn default_mount_point() -> String {
    "secret".to_string()
}

impl VaultConfig {
    /// Create a new VaultConfig
    pub fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            mount_point: default_mount_point(),
            kv_version: SecretEngine::default(),
        }
    }

    /// Build the full API path for a secret
    pub fn build_secret_path(&self, path: &str) -> String {
        match self.kv_version {
            SecretEngine::KvV2 => {
                format!("{}/data/{}", self.mount_point, path)
            }
            SecretEngine::KvV1 => {
                format!("{}/{}", self.mount_point, path)
            }
        }
    }

    /// Build the metadata path for KV v2
    pub fn build_metadata_path(&self, path: &str) -> String {
        match self.kv_version {
            SecretEngine::KvV2 => {
                format!("{}/metadata/{}", self.mount_point, path)
            }
            SecretEngine::KvV1 => {
                format!("{}/{}", self.mount_point, path)
            }
        }
    }

    /// Build the delete path for KV v2
    pub fn build_delete_path(&self, path: &str) -> String {
        match self.kv_version {
            SecretEngine::KvV2 => {
                format!("{}/delete/{}", self.mount_point, path)
            }
            SecretEngine::KvV1 => {
                format!("{}/{}", self.mount_point, path)
            }
        }
    }

    /// Build the list path
    pub fn build_list_path(&self, path: &str) -> String {
        format!("{}/metadata/{}", self.mount_point, path)
    }
}

/// Vault KV Store client trait
#[async_trait]
pub trait VaultClient: Send + Sync {
    /// Retrieve a secret by path
    async fn get_secret(&self, path: &str) -> Result<serde_json::Value, PlatformError>;

    /// Store a secret at the given path
    async fn put_secret(&self, path: &str, data: serde_json::Value) -> Result<(), PlatformError>;

    /// Delete a secret at the given path
    async fn delete_secret(&self, path: &str) -> Result<(), PlatformError>;

    /// List secrets at the given path
    async fn list_secrets(&self, path: &str) -> Result<Vec<String>, PlatformError>;

    /// Get the secret engine version
    fn kv_version(&self) -> &SecretEngine;
}

/// Vault HTTP API client for production use
pub struct HttpVaultClient {
    config: VaultConfig,
    http_client: reqwest::Client,
}

impl HttpVaultClient {
    /// Create a new HttpVaultClient
    pub fn new(config: VaultConfig) -> Result<Self, PlatformError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| PlatformError::Internal(format!("Failed to create HTTP client: {e}")))?;

        info!(
            url = %config.url,
            mount = %config.mount_point,
            kv_version = ?config.kv_version,
            "Vault HTTP client initialized"
        );

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create a builder for HttpVaultClient
    pub fn builder() -> HttpVaultClientBuilder {
        HttpVaultClientBuilder::default()
    }
}

/// Builder for HttpVaultClient
#[derive(Default)]
pub struct HttpVaultClientBuilder {
    url: Option<String>,
    token: Option<String>,
    mount_point: Option<String>,
    kv_version: Option<SecretEngine>,
    timeout_secs: Option<u64>,
}

impl HttpVaultClientBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn mount_point(mut self, mount_point: impl Into<String>) -> Self {
        self.mount_point = Some(mount_point.into());
        self
    }

    pub fn kv_version(mut self, kv_version: SecretEngine) -> Self {
        self.kv_version = Some(kv_version);
        self
    }

    pub fn timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }

    pub fn build(self) -> Result<HttpVaultClient, PlatformError> {
        let config = VaultConfig {
            url: self.url.ok_or_else(|| {
                PlatformError::Validation(platform_error::ValidationError::MissingField(
                    "url".to_string(),
                ))
            })?,
            token: self.token.ok_or_else(|| {
                PlatformError::Validation(platform_error::ValidationError::MissingField(
                    "token".to_string(),
                ))
            })?,
            mount_point: self.mount_point.unwrap_or_else(default_mount_point),
            kv_version: self.kv_version.unwrap_or_default(),
        };

        let timeout = self.timeout_secs.unwrap_or(30);
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| PlatformError::Internal(format!("Failed to create HTTP client: {e}")))?;

        Ok(HttpVaultClient {
            config,
            http_client,
        })
    }
}

/// KV v2 response wrapper
#[derive(Debug, Deserialize)]
struct KvV2Response {
    data: KvV2Data,
}

#[derive(Debug, Deserialize)]
struct KvV2Data {
    data: serde_json::Value,
}

/// KV v2 list response
#[derive(Debug, Deserialize)]
struct KvV2ListResponse {
    data: KvV2ListData,
}

#[derive(Debug, Deserialize)]
struct KvV2ListData {
    keys: Vec<String>,
}

/// KV v1 response (data is at the top level)
#[derive(Debug, Deserialize)]
struct KvV1Response {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct KvV1ListResponse {
    data: KvV1ListData,
}

#[derive(Debug, Deserialize)]
struct KvV1ListData {
    keys: Vec<String>,
}

#[async_trait]
impl VaultClient for HttpVaultClient {
    #[instrument(skip(self), fields(path))]
    async fn get_secret(&self, path: &str) -> Result<serde_json::Value, PlatformError> {
        let api_path = self.config.build_secret_path(path);
        let url = format!("{}/v1/{}", self.config.url, api_path);

        debug!(url = %url, "Fetching secret from Vault");

        let response = self
            .http_client
            .get(&url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("Vault HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                status = %status,
                body = %body,
                path = %path,
                "Vault secret fetch failed"
            );
            return Err(PlatformError::Internal(format!(
                "Vault request failed with status {status}: {body}"
            )));
        }

        match self.config.kv_version {
            SecretEngine::KvV2 => {
                let kv_response: KvV2Response = response.json().await.map_err(|e| {
                    PlatformError::Internal(format!("Failed to parse Vault response: {e}"))
                })?;
                Ok(kv_response.data.data)
            }
            SecretEngine::KvV1 => {
                let kv_response: KvV1Response = response.json().await.map_err(|e| {
                    PlatformError::Internal(format!("Failed to parse Vault response: {e}"))
                })?;
                Ok(kv_response.data)
            }
        }
    }

    #[instrument(skip(self, data), fields(path))]
    async fn put_secret(&self, path: &str, data: serde_json::Value) -> Result<(), PlatformError> {
        let api_path = self.config.build_secret_path(path);
        let url = format!("{}/v1/{}", self.config.url, api_path);

        debug!(url = %url, "Storing secret in Vault");

        let body = match self.config.kv_version {
            SecretEngine::KvV2 => {
                serde_json::json!({ "data": data })
            }
            SecretEngine::KvV1 => data,
        };

        let response = self
            .http_client
            .put(&url)
            .header("X-Vault-Token", &self.config.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("Vault HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                status = %status,
                body = %body,
                path = %path,
                "Vault secret store failed"
            );
            return Err(PlatformError::Internal(format!(
                "Vault request failed with status {status}: {body}"
            )));
        }

        debug!(path = %path, "Secret stored successfully");
        Ok(())
    }

    #[instrument(skip(self), fields(path))]
    async fn delete_secret(&self, path: &str) -> Result<(), PlatformError> {
        let url = match self.config.kv_version {
            SecretEngine::KvV2 => {
                let api_path = self.config.build_delete_path(path);
                format!("{}/v1/{}", self.config.url, api_path)
            }
            SecretEngine::KvV1 => {
                let api_path = self.config.build_secret_path(path);
                format!("{}/v1/{}", self.config.url, api_path)
            }
        };

        debug!(url = %url, "Deleting secret from Vault");

        let response = self
            .http_client
            .post(&url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("Vault HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                status = %status,
                body = %body,
                path = %path,
                "Vault secret delete failed"
            );
            return Err(PlatformError::Internal(format!(
                "Vault request failed with status {status}: {body}"
            )));
        }

        debug!(path = %path, "Secret deleted successfully");
        Ok(())
    }

    #[instrument(skip(self), fields(path))]
    async fn list_secrets(&self, path: &str) -> Result<Vec<String>, PlatformError> {
        let api_path = self.config.build_list_path(path);
        let url = format!("{}/v1/{}?list=true", self.config.url, api_path);

        debug!(url = %url, "Listing secrets from Vault");

        let response = self
            .http_client
            .request(reqwest::Method::GET, &url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("Vault HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            // A 404 means no secrets at this path
            if status == reqwest::StatusCode::NOT_FOUND {
                return Ok(Vec::new());
            }
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                status = %status,
                body = %body,
                path = %path,
                "Vault list secrets failed"
            );
            return Err(PlatformError::Internal(format!(
                "Vault request failed with status {status}: {body}"
            )));
        }

        match self.config.kv_version {
            SecretEngine::KvV2 => {
                let kv_response: KvV2ListResponse = response.json().await.map_err(|e| {
                    PlatformError::Internal(format!("Failed to parse Vault response: {e}"))
                })?;
                Ok(kv_response.data.keys)
            }
            SecretEngine::KvV1 => {
                let kv_response: KvV1ListResponse = response.json().await.map_err(|e| {
                    PlatformError::Internal(format!("Failed to parse Vault response: {e}"))
                })?;
                Ok(kv_response.data.keys)
            }
        }
    }

    fn kv_version(&self) -> &SecretEngine {
        &self.config.kv_version
    }
}

/// In-memory Vault client for development/testing
pub struct InMemoryVaultClient {
    secrets: RwLock<HashMap<String, serde_json::Value>>,
    kv_version: SecretEngine,
}

impl InMemoryVaultClient {
    /// Create a new InMemoryVaultClient
    pub fn new(kv_version: SecretEngine) -> Self {
        info!("Initializing in-memory Vault client");
        Self {
            secrets: RwLock::new(HashMap::new()),
            kv_version,
        }
    }

    /// Create a new InMemoryVaultClient with KvV2
    pub fn new_v2() -> Self {
        Self::new(SecretEngine::KvV2)
    }

    /// Create a new InMemoryVaultClient with KvV1
    pub fn new_v1() -> Self {
        Self::new(SecretEngine::KvV1)
    }
}

impl Default for InMemoryVaultClient {
    fn default() -> Self {
        Self::new_v2()
    }
}

#[async_trait]
impl VaultClient for InMemoryVaultClient {
    #[instrument(skip(self), fields(path))]
    async fn get_secret(&self, path: &str) -> Result<serde_json::Value, PlatformError> {
        let secrets = self.secrets.read().map_err(|e| {
            PlatformError::Internal(format!("Failed to acquire read lock: {e}"))
        })?;

        secrets.get(path).cloned().ok_or_else(|| {
            PlatformError::Internal(format!("Secret not found at path: {path}"))
        })
    }

    #[instrument(skip(self, data), fields(path))]
    async fn put_secret(&self, path: &str, data: serde_json::Value) -> Result<(), PlatformError> {
        let mut secrets = self.secrets.write().map_err(|e| {
            PlatformError::Internal(format!("Failed to acquire write lock: {e}"))
        })?;

        secrets.insert(path.to_string(), data);
        debug!(path = %path, "Secret stored in memory");
        Ok(())
    }

    #[instrument(skip(self), fields(path))]
    async fn delete_secret(&self, path: &str) -> Result<(), PlatformError> {
        let mut secrets = self.secrets.write().map_err(|e| {
            PlatformError::Internal(format!("Failed to acquire write lock: {e}"))
        })?;

        secrets.remove(path);
        debug!(path = %path, "Secret deleted from memory");
        Ok(())
    }

    #[instrument(skip(self), fields(path))]
    async fn list_secrets(&self, path: &str) -> Result<Vec<String>, PlatformError> {
        let secrets = self.secrets.read().map_err(|e| {
            PlatformError::Internal(format!("Failed to acquire read lock: {e}"))
        })?;

        let prefix = format!("{}/", path);
        let keys: Vec<String> = secrets
            .keys()
            .filter(|k| k.starts_with(&prefix) || k == &path)
            .map(|k| {
                // Strip the prefix and return just the key name
                k.strip_prefix(&prefix).unwrap_or(k).to_string()
            })
            .collect();

        Ok(keys)
    }

    fn kv_version(&self) -> &SecretEngine {
        &self.kv_version
    }
}

/// Vault secret paths for the payment platform
pub mod paths {
    /// Data Encryption Key for PII field encryption
    pub const DEK_PATH: &str = "data/encryption/dek";

    /// JWT signing key
    pub const JWT_SIGNING_KEY_PATH: &str = "data/auth/jwt-signing-key";

    /// JWT verification key
    pub const JWT_VERIFICATION_KEY_PATH: &str = "data/auth/jwt-verification-key";

    /// Build connector credentials path
    pub fn connector_credentials(connector_id: &str) -> String {
        format!("data/connectors/{connector_id}/credentials")
    }

    /// Build custom secret path
    pub fn custom(namespace: &str, key: &str) -> String {
        format!("data/{namespace}/{key}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_default_mount_point() {
        let config = VaultConfig::new(
            "https://vault.example.com:8200".to_string(),
            "test-token".to_string(),
        );
        assert_eq!(config.mount_point, "secret");
        assert_eq!(config.kv_version, SecretEngine::KvV2);
    }

    #[test]
    fn test_vault_config_custom_mount_point() {
        let config = VaultConfig {
            url: "https://vault.example.com:8200".to_string(),
            token: "test-token".to_string(),
            mount_point: "kv".to_string(),
            kv_version: SecretEngine::KvV1,
        };
        assert_eq!(config.mount_point, "kv");
        assert_eq!(config.kv_version, SecretEngine::KvV1);
    }

    #[test]
    fn test_build_secret_path_kv_v2() {
        let config = VaultConfig {
            url: "https://vault.example.com:8200".to_string(),
            token: "test-token".to_string(),
            mount_point: "secret".to_string(),
            kv_version: SecretEngine::KvV2,
        };

        assert_eq!(config.build_secret_path("data/auth/jwt-key"), "secret/data/data/auth/jwt-key");
    }

    #[test]
    fn test_build_secret_path_kv_v1() {
        let config = VaultConfig {
            url: "https://vault.example.com:8200".to_string(),
            token: "test-token".to_string(),
            mount_point: "secret".to_string(),
            kv_version: SecretEngine::KvV1,
        };

        assert_eq!(config.build_secret_path("auth/jwt-key"), "secret/auth/jwt-key");
    }

    #[test]
    fn test_build_metadata_path_kv_v2() {
        let config = VaultConfig {
            url: "https://vault.example.com:8200".to_string(),
            token: "test-token".to_string(),
            mount_point: "secret".to_string(),
            kv_version: SecretEngine::KvV2,
        };

        assert_eq!(config.build_metadata_path("data/auth/jwt-key"), "secret/metadata/data/auth/jwt-key");
    }

    #[test]
    fn test_build_delete_path_kv_v2() {
        let config = VaultConfig {
            url: "https://vault.example.com:8200".to_string(),
            token: "test-token".to_string(),
            mount_point: "secret".to_string(),
            kv_version: SecretEngine::KvV2,
        };

        assert_eq!(config.build_delete_path("data/auth/jwt-key"), "secret/delete/data/auth/jwt-key");
    }

    #[test]
    fn test_build_list_path() {
        let config = VaultConfig {
            url: "https://vault.example.com:8200".to_string(),
            token: "test-token".to_string(),
            mount_point: "secret".to_string(),
            kv_version: SecretEngine::KvV2,
        };

        assert_eq!(config.build_list_path("data/auth"), "secret/metadata/data/auth");
    }

    #[test]
    fn test_secret_paths() {
        assert_eq!(paths::DEK_PATH, "data/encryption/dek");
        assert_eq!(paths::JWT_SIGNING_KEY_PATH, "data/auth/jwt-signing-key");
        assert_eq!(paths::JWT_VERIFICATION_KEY_PATH, "data/auth/jwt-verification-key");
        assert_eq!(
            paths::connector_credentials("stripe"),
            "data/connectors/stripe/credentials"
        );
        assert_eq!(paths::custom("analytics", "key"), "data/analytics/key");
    }

    #[tokio::test]
    async fn test_in_memory_vault_client_crud() {
        let client = InMemoryVaultClient::new_v2();

        // Put a secret
        let data = serde_json::json!({
            "key": "value",
            "nested": { "inner": 123 }
        });
        client.put_secret("test/path", data.clone()).await.unwrap();

        // Get the secret
        let retrieved = client.get_secret("test/path").await.unwrap();
        assert_eq!(retrieved, data);

        // List secrets
        let keys = client.list_secrets("test").await.unwrap();
        assert!(keys.contains(&"path".to_string()));

        // Delete the secret
        client.delete_secret("test/path").await.unwrap();

        // Verify deletion
        let result = client.get_secret("test/path").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_vault_client_nonexistent_secret() {
        let client = InMemoryVaultClient::new_v2();

        let result = client.get_secret("nonexistent/path").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_vault_client_overwrite() {
        let client = InMemoryVaultClient::new_v2();

        let data1 = serde_json::json!({ "version": 1 });
        let data2 = serde_json::json!({ "version": 2 });

        client.put_secret("test/path", data1).await.unwrap();
        client.put_secret("test/path", data2.clone()).await.unwrap();

        let retrieved = client.get_secret("test/path").await.unwrap();
        assert_eq!(retrieved, data2);
    }

    #[tokio::test]
    async fn test_in_memory_vault_client_list_empty() {
        let client = InMemoryVaultClient::new_v2();

        let keys = client.list_secrets("empty/path").await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_vault_client_list_multiple() {
        let client = InMemoryVaultClient::new_v2();

        client.put_secret("data/auth/key1", serde_json::json!("v1")).await.unwrap();
        client.put_secret("data/auth/key2", serde_json::json!("v2")).await.unwrap();
        client.put_secret("data/encryption/key3", serde_json::json!("v3")).await.unwrap();

        let keys = client.list_secrets("data/auth").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_vault_client_kv_version() {
        let client_v1 = InMemoryVaultClient::new_v1();
        assert_eq!(client_v1.kv_version(), &SecretEngine::KvV1);

        let client_v2 = InMemoryVaultClient::new_v2();
        assert_eq!(client_v2.kv_version(), &SecretEngine::KvV2);

        let client_default = InMemoryVaultClient::default();
        assert_eq!(client_default.kv_version(), &SecretEngine::KvV2);
    }

    #[test]
    fn test_vault_config_builder() {
        let client = HttpVaultClient::builder()
            .url("https://vault.example.com:8200")
            .token("test-token")
            .mount_point("kv")
            .kv_version(SecretEngine::KvV1)
            .timeout_secs(60)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config.url, "https://vault.example.com:8200");
        assert_eq!(client.config.token, "test-token");
        assert_eq!(client.config.mount_point, "kv");
        assert_eq!(client.config.kv_version, SecretEngine::KvV1);
    }

    #[test]
    fn test_vault_config_builder_missing_url() {
        let client = HttpVaultClient::builder()
            .token("test-token")
            .build();

        assert!(client.is_err());
    }

    #[test]
    fn test_vault_config_builder_missing_token() {
        let client = HttpVaultClient::builder()
            .url("https://vault.example.com:8200")
            .build();

        assert!(client.is_err());
    }

    #[test]
    fn test_secret_engine_serialization() {
        let kv_v1 = SecretEngine::KvV1;
        let kv_v2 = SecretEngine::KvV2;

        assert_eq!(serde_json::to_string(&kv_v1).unwrap(), "\"kv-v1\"");
        assert_eq!(serde_json::to_string(&kv_v2).unwrap(), "\"kv-v2\"");
    }

    #[test]
    fn test_secret_engine_deserialization() {
        let kv_v1: SecretEngine = serde_json::from_str("\"kv-v1\"").unwrap();
        let kv_v2: SecretEngine = serde_json::from_str("\"kv-v2\"").unwrap();

        assert_eq!(kv_v1, SecretEngine::KvV1);
        assert_eq!(kv_v2, SecretEngine::KvV2);
    }
}
