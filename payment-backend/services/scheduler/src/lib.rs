//! Background job scheduler — library crate.
//! In-process cron-style scheduling with leader election (LEADER-001).

pub mod domain;
pub mod commands;
pub mod events;
pub mod repository;
pub mod pipeline;
