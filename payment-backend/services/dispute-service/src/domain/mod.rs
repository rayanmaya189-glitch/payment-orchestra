//! Dispute Management domain model — BC-10
//!
//! Event-sourced ChargebackCase aggregate with representment lifecycle.
//!
//! Per CONVENTIONS.md: one concept per file.

pub mod chargeback_case;
pub mod error;
pub mod status;

pub use chargeback_case::*;
pub use error::*;
pub use status::*;
