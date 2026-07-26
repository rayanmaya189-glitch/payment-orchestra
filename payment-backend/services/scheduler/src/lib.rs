//! Background job scheduler — library crate.
//! In-process cron-style scheduling with leader election (LEADER-001).

#![allow(clippy::result_large_err)]
// Allow unused code in API/scaffolding modules — these are wired up by
// the binary (`main.rs`) or will be consumed by downstream integration tests.
// The binary only uses `domain` and `repository` directly; the rest are
// structured for future gRPC service composition.
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
