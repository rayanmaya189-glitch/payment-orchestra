//! Gateway Profile repository — trait + in-memory + PostgreSQL implementations.

pub mod traits;
pub mod in_memory;
pub mod pg;
pub use traits::*;
pub use in_memory::*;
pub use pg::PostgresConnectorGatewayRepository;
