//! Graceful shutdown orchestration per SRS BIZ-052, SHUTDOWN-003.
//!
//! Handles both SIGTERM (K8s) and Ctrl+C (dev), with configurable drain timeout.
//! Returns a future that resolves when shutdown is complete.

use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};

/// Shutdown configuration per SRS SHUTDOWN-003.
pub struct ShutdownConfig {
    /// Drain timeout for in-flight requests (15s for checkout-critical, 60s for others).
    pub drain_timeout: Duration,
    /// Service name for structured logging.
    pub service_name: String,
}

impl ShutdownConfig {
    /// Create config for a critical-path service (e.g., orchestration, connector-gateway).
    pub fn critical(service_name: &str) -> Self {
        Self {
            drain_timeout: Duration::from_secs(15),
            service_name: service_name.to_string(),
        }
    }

    /// Create config for a non-critical service.
    pub fn standard(service_name: &str) -> Self {
        Self {
            drain_timeout: Duration::from_secs(60),
            service_name: service_name.to_string(),
        }
    }
}

/// Returns a shutdown future that resolves on SIGTERM or Ctrl+C.
/// After signal, logs the drain timeout and waits for `axum::serve` to drain.
pub fn shutdown_signal(config: ShutdownConfig) -> impl std::future::Future<Output = ()> {
    async move {
        let ctrl_c = tokio::signal::ctrl_c();
        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!(
                    service = %config.service_name,
                    drain_timeout_secs = config.drain_timeout.as_secs(),
                    "Shutdown signal (Ctrl+C) received, draining in-flight requests..."
                );
            }
            _ = sigterm.recv() => {
                tracing::info!(
                    service = %config.service_name,
                    drain_timeout_secs = config.drain_timeout.as_secs(),
                    "Shutdown signal (SIGTERM) received, draining in-flight requests..."
                );
            }
        }

        // Give in-flight requests time to complete (drain timeout)
        // axum::serve's with_graceful_shutdown handles the actual drain
        tokio::time::sleep(config.drain_timeout).await;

        tracing::info!(
            service = %config.service_name,
            "Drain period complete, shutting down"
        );
    }
}
