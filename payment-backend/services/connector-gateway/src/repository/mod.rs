//! Gateway Profile repository — trait + in-memory implementation.

pub mod traits;
pub mod in_memory;
pub use traits::*;
pub use in_memory::*;
