//! Command processing pipeline for reconciliation-service.

use tracing::{info, error};

use crate::commands::*;
use crate::domain::*;
use crate::queries::*;

/// Pipeline middleware for command execution.
pub struct ReconciliationPipeline<H: CommandHandler, Q: QueryHandler> {
    handler: H,
    query_handler: Q,
}

impl<H: CommandHandler, Q: QueryHandler> ReconciliationPipeline<H, Q> {
    pub fn new(handler: H, query_handler: Q) -> Self {
        Self { handler, query_handler }
    }
}

impl<H: CommandHandler, Q: QueryHandler> ReconciliationPipeline<H, Q> {
    pub async fn ingest_settlement_batch(&self, cmd: IngestSettlementBatch) -> Result<IngestBatchResult, ReconciliationError> {
        info!(acquirer_link_id = %cmd.acquirer_link_id, "Ingesting settlement batch");
        let result = self.handler.ingest_settlement_batch(cmd).await;
        if let Err(e) = &result {
            error!(error = %e, "Failed to ingest settlement batch");
        }
        result
    }

    pub async fn process_batch_matching(&self, cmd: ProcessBatchMatching) -> Result<MatchingResult, ReconciliationError> {
        info!(batch_id = %cmd.settlement_batch_id, pi_count = cmd.payment_intents.len(), "Processing batch matching");
        let result = self.handler.process_batch_matching(cmd).await;
        match &result {
            Ok(r) => info!(matched = r.matched_count, unmatched = r.unmatched_count, "Batch matching completed"),
            Err(e) => error!(error = %e, "Batch matching failed"),
        }
        result
    }

    pub async fn track_fee_variance(&self, cmd: TrackFeeVariance) -> Result<FeeVarianceResult, ReconciliationError> {
        info!(pi_id = %cmd.payment_intent_id, estimated = cmd.estimated_fee_minor, actual = cmd.actual_fee_minor, "Tracking fee variance");
        self.handler.track_fee_variance(cmd).await
    }

    // ── Query passthrough ─────────────────────────────────────────────────

    pub async fn get_settlement_batch(&self, query: GetSettlementBatchQuery) -> Result<Option<SettlementBatch>, ReconciliationError> {
        self.query_handler.get_settlement_batch(query).await
    }

    pub async fn get_unmatched_records(&self, query: GetUnmatchedRecordsQuery) -> Result<Vec<SettlementRecord>, ReconciliationError> {
        self.query_handler.get_unmatched_records(query).await
    }
}
