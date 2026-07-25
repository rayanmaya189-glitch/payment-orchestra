//! Command handlers for BC-01 Operator Management.
//!
//! Per CONVENTIONS.md:
//! - `types.rs` — Command input/result structs
//! - `handler.rs` — CommandHandler trait + OperatorCommandHandler impl
//! - `register.rs` — register_impl
//! - `status.rs` — update_status_impl

pub mod handler;
pub mod types;
pub(crate) mod register;
pub(crate) mod status;

#[cfg(test)]
pub(crate) mod tests;

// Handler re-exports types via `pub use super::types::*;`
pub use handler::*;
