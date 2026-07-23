//! Analytics Service repository — in-memory ClickHouse simulation

pub mod traits;
pub mod in_memory;
pub use traits::*;
pub use in_memory::*;
