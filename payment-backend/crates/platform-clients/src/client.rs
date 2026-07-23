//! Base gRPC client connection management.
//!
//! Provides the [`ServiceConnection`] helper that manages a tonic channel
//! to a target service with automatic reconnection and timeout support.

use std::time::Duration;

use tonic::transport::{Channel, Endpoint};

/// Errors that can occur when connecting to a service.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Invalid URI for {service}: {message}")]
    InvalidUri {
        service: String,
        message: String,
    },

    #[error("Failed to connect to {service} at {addr}: {source}")]
    ConnectionFailed {
        service: String,
        addr: String,
        #[source]
        source: tonic::transport::Error,
    },

    #[error("gRPC call to {service} failed: {source}")]
    RpcFailed {
        service: String,
        #[source]
        source: tonic::Status,
    },

    #[error("Service {service} not found in etcd registry")]
    ServiceNotFound { service: String },
}

/// A managed connection to a target service.
///
/// Wraps a tonic `Channel` with automatic reconnection.
/// The channel uses HTTP/2 multiplexing, so a single channel
/// can handle many concurrent RPCs.
#[derive(Debug, Clone)]
pub struct ServiceConnection {
    /// Service name for logging
    pub service_name: String,
    /// Tonic channel to the target service
    channel: Channel,
}

impl ServiceConnection {
    /// Create a new connection to a service at the given address.
    ///
    /// `addr` should be like `http://127.0.0.1:9002`.
    /// The channel includes connect and request timeouts.
    pub async fn connect(service_name: &str, addr: &str) -> Result<Self, ClientError> {
        let endpoint = match Endpoint::from_shared(addr.to_string()) {
            Ok(e) => e
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(30)),
            Err(e) => {
                return Err(ClientError::InvalidUri {
                    service: service_name.to_string(),
                    message: e.to_string(),
                });
            }
        };

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| ClientError::ConnectionFailed {
                service: service_name.to_string(),
                addr: addr.to_string(),
                source: e,
            })?;

        Ok(Self {
            service_name: service_name.to_string(),
            channel,
        })
    }

    /// Create a lazy connection that retries on first use.
    ///
    /// Unlike [`connect`], this does not connect immediately.
    /// The connection is established on the first RPC call.
    pub fn connect_lazy(service_name: &str, addr: &str) -> Result<Self, ClientError> {
        let endpoint = match Endpoint::from_shared(addr.to_string()) {
            Ok(e) => e
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(30)),
            Err(e) => {
                return Err(ClientError::InvalidUri {
                    service: service_name.to_string(),
                    message: e.to_string(),
                });
            }
        };

        let channel = endpoint.connect_lazy();

        Ok(Self {
            service_name: service_name.to_string(),
            channel,
        })
    }

    /// Get a reference to the underlying tonic channel.
    pub fn channel(&self) -> &Channel {
        &self.channel
    }
}
