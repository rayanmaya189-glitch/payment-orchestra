//! In-process NATS-compatible messaging channels.
//! Implements ADR-004: in-process NATS-compatible API (tokio broadcast/mpsc).

pub mod event_bus;
pub mod subject;
pub mod envelope;
pub mod consumer;
pub mod dlq;
