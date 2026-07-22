//! Saga Coordinator — Library crate for multi-step cross-aggregate workflows.
//! BC-17: Durable state machine for compensation-capable business processes.

pub mod domain;
pub mod commands;
pub mod queries;
pub mod events;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;
