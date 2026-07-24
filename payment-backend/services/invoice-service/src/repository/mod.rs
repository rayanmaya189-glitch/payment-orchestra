//! Repository interface and in-memory implementation for invoice-service.

pub mod event_sourced;
pub mod pg;
pub mod traits;
pub mod in_memory;
pub use traits::*;
pub use in_memory::*;
