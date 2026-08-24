//! Server Bootstrap — helpers for starting gRPC servers with etcd registration.
//!
//! Usage pattern for services WITH gRPC implementations:
//! ```rust,ignore
//! let mut runner = ServerRunner::new("my-service", 9010, 9110).await?;
//! let server = tonic::transport::Server::builder()
//!     .add_service(MyServiceServer::new(MyServiceImpl))
//!     .serve(runner.grpc_addr);
//! tokio::select! {
//!     result = server => { result?; }
//!     _ = tokio::signal::ctrl_c() => {
//!         tracing::info!("Shutdown signal received");
//!     }
//! }
//! runner.deregister().await;
//! ```
//!
//! Usage pattern for services WITHOUT gRPC (scaffolding):
//! ```rust,ignore
//! let mut runner = ServerRunner::new("my-service", 9010, 9110).await?;
//! runner.wait_for_shutdown().await?;
//! runner.deregister().await;
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use tokio::signal;

use crate::{create_service_instance, Registry, RegistryConfig};

/// Standard server runner with etcd registration and graceful shutdown.
pub struct ServerRunner {
    pub service_name: String,
    pub instance_id: String,
    pub grpc_addr: SocketAddr,
    pub health_addr: SocketAddr,
    registry: Option<Registry>,
    _shutdown_timeout: Duration,
}

impl ServerRunner {
    /// Create a new server runner and register with etcd.
    ///
    /// Port resolution priority:
    /// 1. `{service_name}_grpc_port` / `{service_name}_health_port` env vars (per-service)
    /// 2. Generic `GRPC_PORT` / `HEALTH_PORT` env vars (from `ServiceConfig::from_env`)
    /// 3. `default_grpc_port` / `default_health_port` fallback
    ///
    /// Service name hyphens (`-`) are converted to underscores (`_`) for env var lookup.
    /// Example: `"operator-service"` reads `operator_service_grpc_port`.
    pub async fn new(
        service_name: &str,
        default_grpc_port: u16,
        default_health_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load shared config once (used for etcd, shutdown timeout, and generic port fallback)
        let config = platform_config::config::ServiceConfig::from_env().unwrap_or_default();

        // Build per-service env var names: replace '-' with '_'
        // Example: "operator-service" -> "operator_service_grpc_port"
        let env_key = service_name.replace('-', "_");
        let grpc_env = format!("{}_grpc_port", env_key);
        let health_env = format!("{}_health_port", env_key);

        // Port resolution priority:
        //   1. Per-service env var (e.g., operator_service_grpc_port)
        //   2. Hardcoded default_grpc_port / default_health_port parameter
        //
        // Note: Generic GRPC_PORT env var is NOT used as fallback because
        // ServiceConfig::default().grpc_port = 9000, which would shadow the
        // per-service defaults (9001, 9002, ...) whenever no env var is set.
        let grpc_port = std::env::var(&grpc_env)
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(default_grpc_port);

        let health_port = std::env::var(&health_env)
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(default_health_port);

        let grpc_addr: SocketAddr = format!("0.0.0.0:{}", grpc_port).parse()?;
        let health_addr: SocketAddr = format!("0.0.0.0:{}", health_port).parse()?;

        // Connect to etcd and register
        let instance = create_service_instance(service_name, grpc_port, health_port);
        let instance_id = instance.instance_id.clone();

        let registry_config = RegistryConfig {
            etcd_endpoints: if config.etcd_endpoints.is_empty() {
                vec!["http://localhost:2379".into()]
            } else {
                config.etcd_endpoints.clone()
            },
            lease_ttl_secs: config.etcd_lease_ttl_secs.max(10),
            ..Default::default()
        };

        let registry = match Registry::connect(registry_config).await {
            Ok(mut r) => {
                if let Err(e) = r.register(instance).await {
                    tracing::warn!(error = %e, "Failed to register with etcd, continuing without discovery");
                    None
                } else {
                    Some(r)
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to connect to etcd, continuing without discovery");
                None
            }
        };

        tracing::info!(
            service = %service_name,
            grpc = %grpc_addr,
            health = %health_addr,
            discovery_enabled = registry.is_some(),
            "Service started"
        );

        Ok(Self {
            service_name: service_name.to_string(),
            instance_id,
            grpc_addr,
            health_addr,
            registry,
            _shutdown_timeout: Duration::from_secs(config.graceful_shutdown_timeout_secs),
        })
    }

    /// Wait for Ctrl+C shutdown signal.
    /// Use this when the service doesn't have a gRPC server but still needs to register in etcd.
    pub async fn wait_for_shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        signal::ctrl_c().await?;
        tracing::info!(service = %self.service_name, "Shutdown signal received");
        Ok(())
    }

    /// Deregister from etcd (called during shutdown).
    /// Clears the registry to prevent false-positive `Drop` warning.
    pub async fn deregister(&mut self) {
        if let Some(ref mut registry) = self.registry {
            if let Err(e) = registry.deregister().await {
                tracing::warn!(
                    error = %e,
                    service = %self.service_name,
                    "Failed to deregister from etcd"
                );
            }
        }
        // Clear registry so Drop impl doesn't false-positive warn
        self.registry = None;
    }
}

impl Drop for ServerRunner {
    fn drop(&mut self) {
        if self.registry.is_some() {
            tracing::warn!(
                service = %self.service_name,
                "ServerRunner dropped without explicit deregister"
            );
        }
    }
}
