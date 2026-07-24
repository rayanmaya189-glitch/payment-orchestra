//! Platform Clients — typed gRPC client wrappers for all platform services.
//!
//! Each client wraps the tonic-generated `*ServiceClient` from `platform-proto`
//! and provides a clean, typed API for inter-service communication.

// gRPC client functions return platform_error::PlatformError which aggregates
// all possible failure modes into a single large enum. This is intentional.
#![allow(clippy::result_large_err)]

pub mod client;
pub mod iam;
pub mod risk;
pub mod merchant_link;
pub mod orchestration;
pub mod connector;
pub mod operator;
pub mod compliance;
pub mod invoice;
pub mod subscription;
pub mod dispute;
pub mod reconciliation;
pub mod payment_link;
pub mod notification;
pub mod document;
pub mod analytics;
pub mod ai_assistant;
pub mod saga_coordinator;

pub use client::ClientError;
pub use client::ServiceConnection;
