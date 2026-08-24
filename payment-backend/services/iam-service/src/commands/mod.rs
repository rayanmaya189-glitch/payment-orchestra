//! Command handlers for BC-02 Identity & Access Management.
//!
//! Per CONVENTIONS.md:
//! - `types.rs` — Command input/result structs
//! - `handler.rs` — CommandHandler trait + IamCommandHandler impl
//! - `auth.rs` — authenticate_impl
//! - `api_key.rs` — API key operations impl
//! - `maker_checker.rs` — Maker/checker operations impl

pub mod handler;
pub mod types;
pub(crate) mod auth;
pub(crate) mod api_key;
pub(crate) mod maker_checker;

#[cfg(test)]
pub(crate) mod tests;

pub use handler::*;
