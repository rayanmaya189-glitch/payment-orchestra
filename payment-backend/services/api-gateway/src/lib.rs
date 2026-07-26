#![allow(clippy::result_large_err)]
// Allow unused code in API/scaffolding modules — these are structured for
// future gRPC service composition. The binary (`main.rs`) only uses
// `commands`, `queries`, and `repository` directly.
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
