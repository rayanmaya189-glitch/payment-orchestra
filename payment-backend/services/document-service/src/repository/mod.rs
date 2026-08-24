//! Document Management repository — BC-13

pub mod traits;
pub mod in_memory;
pub mod pg;
pub use traits::*;
pub use in_memory::*;
pub use pg::*;
