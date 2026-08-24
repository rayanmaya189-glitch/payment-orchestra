//! Outbox Relay — Background task that polls outbox and publishes to in-process NATS channels.
//! Implements ADR-011 (Transactional Outbox) within each event-sourced service.

#![allow(clippy::result_large_err)]
// Allow unused code in API/scaffolding modules — these are structured for
// future gRPC service composition. The binary (`main.rs`) only uses
// `domain` and `repository` directly.
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod domain;
pub mod entities;
pub mod commands;
pub mod queries;
pub mod events;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;
