//! SeaORM entity models for the iam-service.
//!
//! Each entity in its own file to maintain clear separation.

pub mod principal;
pub mod pending_change;
pub mod api_key;
pub use principal::*;
