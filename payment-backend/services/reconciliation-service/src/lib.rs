//! reconciliation-service — Settlement & Reconciliation Engine.
//!
//! This service handles:
//! - **SettlementBatch** aggregate (event-sourced): File ingestion, record matching
//! - **LedgerEntry** (append-only): Double-entry accounting for settlement tracking
//! - **SettlementExpectation** (CRUD + events): T+N settlement timing tracking
//! - **FeeVariance** (CRUD + events): Estimated vs. actual fee discrepancy tracking
//!
//! ## Architecture Context
//! This module runs within the modular monolith alongside all other modules.
//!
//! ## Pure Router
//! The platform is a routing and orchestration layer only. Funds flow directly
//! between the customer, the payment gateway, and the merchant bank account.

pub mod domain;
pub mod commands;
pub mod queries;
pub mod events;
pub mod entities;
pub mod repository;
pub mod api;
pub mod pipeline;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use domain::{
    SettlementBatch, SettlementRecord, LedgerEntry, SettlementExpectation, FeeVariance,
    SettlementMatchOutcome, MatchResult, BatchStatus, ExpectationStatus, FeeVarianceStatus,
    ReconciliationMatcher, PaymentIntentRef, Money, SettlementFormat, EntryType,
    ReconciliationError,
};
pub use commands::{
    CommandHandler, ReconciliationCommandHandler,
    IngestSettlementBatch, ProcessBatchMatching, TrackFeeVariance,
    IngestBatchResult, MatchingResult, FeeVarianceResult,
};
pub use events::{
    ReconciliationEvent,
    SettlementBatchIngested, SettlementRecordMatched, SettlementRecordUnmatched,
    FeeVarianceDetected, FeeVarianceResolved,
    SettlementExpected, SettlementOverdue, SettlementCompleted,
    LedgerEntryCreated,
};
pub use repository::{
    SettlementBatchRepository, LedgerEntryRepository,
    SettlementExpectationRepository, FeeVarianceRepository,
    InMemoryReconciliationRepository,
};
pub use queries::{
    QueryHandler, ReconciliationQueryHandler,
    GetSettlementBatchQuery, GetUnmatchedRecordsQuery, GetFeeVarianceQuery,
};
pub use api::ReconciliationApi;
pub use pipeline::ReconciliationPipeline;
