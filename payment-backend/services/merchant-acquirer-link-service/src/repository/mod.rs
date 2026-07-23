//! Repository interface and in-memory implementation for MerchantAcquirerLink aggregate.

pub mod traits;
pub mod in_memory;
pub use traits::*;
pub use in_memory::*;
