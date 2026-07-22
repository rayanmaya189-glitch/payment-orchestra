//! Reconciliation-service API — handler exports for the modular monolith.

pub use crate::commands::{CommandHandler, ReconciliationCommandHandler};
pub use crate::queries::{QueryHandler, ReconciliationQueryHandler};
pub use crate::domain::{SettlementBatch, SettlementRecord, LedgerEntry, FeeVariance, SettlementExpectation};

/// Combined service interface for the API gateway.
pub struct ReconciliationApi {
    pub commands: Box<dyn CommandHandler>,
    pub queries: Box<dyn QueryHandler>,
}
