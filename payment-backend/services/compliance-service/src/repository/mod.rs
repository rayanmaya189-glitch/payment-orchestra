//! Repository interface and implementations for compliance-service.

pub mod traits;
pub mod in_memory;
pub mod pg;
pub use traits::*;
pub use in_memory::*;
pub use pg::PostgresComplianceRepository;
