//! Background job scheduler — library crate.
//! In-process cron-style scheduling with leader election (LEADER-001).

pub mod domain;
pub mod commands;
pub mod queries;
pub mod events;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;
