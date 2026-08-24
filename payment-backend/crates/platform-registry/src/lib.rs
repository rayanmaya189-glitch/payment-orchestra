//! Platform Registry — etcd-based service discovery for all microservices.
//!
//! Each service registers itself on startup with:
//! - A unique key: `/platform/services/{service_name}/{instance_id}`
//! - A lease (TTL) for automatic deregistration on crash
//! - Metadata: gRPC address, health endpoint, version

use std::collections::HashMap;
use std::time::Duration;

use etcd_client::PutOptions;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use uuid::Uuid;

// ─── Registry Errors ────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("etcd connection failed: {0}")]
    ConnectionFailed(String),

    #[error("etcd operation failed: {0}")]
    OperationFailed(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("service not found: {0}")]
    ServiceNotFound(String),

    #[error("lease expired: {0}")]
    LeaseExpired(String),
}

// ─── Service Instance Metadata ──────────────────────────────────────────────

/// Metadata registered in etcd for each service instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// Unique instance ID (UUIDv7)
    pub instance_id: String,
    /// Service name (e.g., "operator-service", "orchestration-service")
    pub service_name: String,
    /// gRPC listen address (e.g., "0.0.0.0:9010")
    pub grpc_address: String,
    /// Health check endpoint (e.g., "0.0.0.0:9110")
    pub health_address: String,
    /// Protocol version for compatibility checking
    pub protocol_version: String,
    /// Additional metadata (labels, region, etc.)
    pub metadata: HashMap<String, String>,
}

/// Service registration configuration.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// etcd endpoint(s)
    pub etcd_endpoints: Vec<String>,
    /// TTL for service lease in seconds
    pub lease_ttl_secs: i64,
    /// Key prefix in etcd (default: "/platform/services/")
    pub key_prefix: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            etcd_endpoints: vec!["http://localhost:2379".to_string()],
            lease_ttl_secs: 30,
            key_prefix: "/platform/services/".to_string(),
        }
    }
}

// ─── Registry Client ────────────────────────────────────────────────────────

/// Etcd-based service registry client.
pub struct Registry {
    client: etcd_client::Client,
    config: RegistryConfig,
    instance: Option<ServiceInstance>,
    lease_id: Option<i64>,
    _keepalive_handle: Option<JoinHandle<()>>,
}

impl Registry {
    /// Create a new registry client and connect to etcd.
    pub async fn connect(config: RegistryConfig) -> Result<Self, RegistryError> {
        let client = etcd_client::Client::connect(config.etcd_endpoints.clone(), None)
            .await
            .map_err(|e| RegistryError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            client,
            config,
            instance: None,
            lease_id: None,
            _keepalive_handle: None,
        })
    }

    /// Register this service instance in etcd with a lease.
    /// The lease ensures the instance is automatically deregistered on crash.
    pub async fn register(&mut self, instance: ServiceInstance) -> Result<(), RegistryError> {
        let key = format!(
            "{}{}/{}",
            self.config.key_prefix, instance.service_name, instance.instance_id
        );

        // Create a lease with TTL
        let lease = self
            .client
            .lease_client()
            .grant(self.config.lease_ttl_secs, None)
            .await
            .map_err(|e| RegistryError::OperationFailed(e.to_string()))?;

        let lease_id = lease.id();

        // Spawn a background task to keep the lease alive
        let mut lease_client = self.client.lease_client();
        let keepalive_lease_id = lease_id;
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if let Err(e) = lease_client.keep_alive(keepalive_lease_id).await {
                    tracing::warn!(lease_id = %keepalive_lease_id, error = %e, "Lease keepalive failed");
                    break;
                }
            }
        });

        // Serialize instance metadata as JSON
        let value = serde_json::to_string(&instance)
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        // Store instance data with lease (auto-expires on crash)
        let opts = PutOptions::new().with_lease(lease_id);
        self.client
            .kv_client()
            .put(key, value, Some(opts))
            .await
            .map_err(|e| RegistryError::OperationFailed(e.to_string()))?;

        self.instance = Some(instance);
        self.lease_id = Some(lease_id);
        self._keepalive_handle = Some(handle);

        tracing::info!(
            service = %self.instance.as_ref().map(|i| &i.service_name).unwrap_or(&"unknown".to_string()),
            instance_id = %self.instance.as_ref().map(|i| &i.instance_id).unwrap_or(&"unknown".to_string()),
            lease_id = %lease_id,
            "Service registered in etcd"
        );

        Ok(())
    }

    /// Resolve a service's gRPC address from etcd by service name.
    /// Returns the first registered instance's gRPC address.
    pub async fn resolve_service(&self, service_name: &str) -> Result<ServiceInstance, RegistryError> {
        let key = format!("{}{}", self.config.key_prefix, service_name);

        let resp = self
            .client
            .kv_client()
            .get(key, Some(etcd_client::GetOptions::new().with_prefix()))
            .await
            .map_err(|e| RegistryError::OperationFailed(e.to_string()))?;

        let kv = resp
            .kvs()
            .first()
            .ok_or_else(|| RegistryError::ServiceNotFound(format!("No instance of {} registered", service_name)))?;

        let instance: ServiceInstance = serde_json::from_slice(kv.value())
            .map_err(|e| RegistryError::Serialization(e.to_string()))?;

        Ok(instance)
    }

    /// Deregister this service instance from etcd.
    pub async fn deregister(&mut self) -> Result<(), RegistryError> {
        if let Some(ref instance) = self.instance {
            let key = format!(
                "{}{}/{}",
                self.config.key_prefix, instance.service_name, instance.instance_id
            );

            self.client
                .kv_client()
                .delete(key, None)
                .await
                .map_err(|e| RegistryError::OperationFailed(e.to_string()))?;

            // Revoke lease
            if let Some(lease_id) = self.lease_id {
                self.client
                    .lease_client()
                    .revoke(lease_id)
                    .await
                    .map_err(|e| RegistryError::OperationFailed(e.to_string()))?;

                // Stop the keepalive task
                if let Some(handle) = self._keepalive_handle.take() {
                    handle.abort();
                }
            }

            tracing::info!(
                service = %instance.service_name,
                instance_id = %instance.instance_id,
                "Service deregistered from etcd"
            );

            self.instance = None;
            self.lease_id = None;
        }

        Ok(())
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        if self.instance.is_some() {
            tracing::warn!("Registry dropped without explicit deregister");
            if let Some(handle) = self._keepalive_handle.take() {
                handle.abort();
            }
        }
    }
}

// ─── Helper function to create standard ServiceInstance ─────────────────────

/// Create a standard ServiceInstance for any service.
pub fn create_service_instance(
    service_name: &str,
    grpc_port: u16,
    health_port: u16,
) -> ServiceInstance {
    ServiceInstance {
        instance_id: Uuid::now_v7().to_string(),
        service_name: service_name.to_string(),
        grpc_address: format!("0.0.0.0:{}", grpc_port),
        health_address: format!("0.0.0.0:{}", health_port),
        protocol_version: env!("CARGO_PKG_VERSION").to_string(),
        metadata: HashMap::new(),
    }
}

pub mod bootstrap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service_instance() {
        let instance = create_service_instance("test-service", 9001, 9101);
        assert_eq!(instance.service_name, "test-service");
        assert_eq!(instance.grpc_address, "0.0.0.0:9001");
        assert_eq!(instance.health_address, "0.0.0.0:9101");
        assert!(!instance.instance_id.is_empty());
    }

    #[test]
    fn test_service_instance_json_roundtrip() {
        let instance = create_service_instance("test", 9001, 9101);
        let json = serde_json::to_string(&instance).unwrap();
        let decoded: ServiceInstance = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.service_name, instance.service_name);
        assert_eq!(decoded.grpc_address, instance.grpc_address);
    }

    #[test]
    fn test_default_registry_config() {
        let config = RegistryConfig::default();
        assert_eq!(config.etcd_endpoints, vec!["http://localhost:2379"]);
        assert_eq!(config.lease_ttl_secs, 30);
        assert_eq!(config.key_prefix, "/platform/services/");
    }
}
