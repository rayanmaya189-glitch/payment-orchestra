//! Platform Clients — typed gRPC client wrappers for all platform services.
//!
//! Each client wraps the tonic-generated `*ServiceClient` from `platform-proto`
//! and provides a clean, typed API for inter-service communication.

pub mod client;
pub mod iam;
pub mod risk;
pub mod merchant_link;
pub mod orchestration;

pub use client::ClientError;
