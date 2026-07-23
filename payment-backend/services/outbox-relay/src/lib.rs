//! Outbox Relay — Background task that polls outbox and publishes to in-process NATS channels.
//! Implements ADR-011 (Transactional Outbox) within each event-sourced service.

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
