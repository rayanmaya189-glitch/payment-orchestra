//! Onboarding repository

pub mod pg;
pub mod traits;
pub mod in_memory;
pub use traits::*;
pub use in_memory::*;
pub use pg::PostgresOnboardingRepository;
