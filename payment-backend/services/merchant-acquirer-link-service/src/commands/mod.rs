//! Command handlers for BYOK Core — MerchantAcquirerLink lifecycle management.
//!
//! Per CONVENTIONS.md:
//! - `types.rs` — Command input/result structs
//! - `handler.rs` — CommandHandler trait + LinkCommandHandler impl
//! - `link.rs` — create_link_impl
//! - `credential.rs` — credential operations impl

pub mod handler;
pub mod types;
pub(crate) mod link;
pub(crate) mod credential;

#[cfg(test)]
pub(crate) mod tests;

// Handler re-exports types via `pub use super::types::*;`
pub use handler::*;
