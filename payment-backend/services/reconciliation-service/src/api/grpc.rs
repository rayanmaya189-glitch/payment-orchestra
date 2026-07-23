//! gRPC service implementation for reconciliation-service (BC-09).
//! Translates between protobuf types and domain types for settlement matching.

use chrono::{DateTime, Utc};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::*;
use crate::queries::{self as query_types, QueryHandler};

use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationRequest, PaginationResponse};
use platform_proto::reconciliation::reconciliation_service_server::ReconciliationService;
use platform_proto::reconciliation::*;

pub struct ReconciliationGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> ReconciliationGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> ReconciliationService for ReconciliationGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn ingest_settlement_file(
        &self,
        request: Request<IngestSettlementFileRequest>,
    ) -> Result<Response<IngestSettlementFileResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let acquirer_link_id = parse_uuid(&req.acquirer_link_id, "acquirer_link_id")?;

        let file_format = match req.file_format.as_str() {
            "csv" => SettlementFormat::Csv,
            "xml" => SettlementFormat::Csv, // Map XML to CSV-based parsing for Phase 1
            "json" => SettlementFormat::PollingApi,
            "sftp" => SettlementFormat::Sftp,
            "webhook" => SettlementFormat::Webhook,
            _ => SettlementFormat::Csv,
        };

        let cmd = commands::IngestSettlementBatch {
            operator_id,
            acquirer_link_id,
            raw_file: req.file_content.into_bytes(),
            file_format,
        };

        match self.commands.ingest_settlement_batch(cmd).await {
            Ok(result) => {
                Ok(Response::new(IngestSettlementFileResponse {
                    batch_id: result.settlement_batch_id.to_string(),
                    record_count: result.total_records,
                    ingested_at: Some(Timestamp {
                        unix_ms: Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(reconciliation_error_to_status(e)),
        }
    }

    async fn get_reconciliation_exceptions(
        &self,
        request: Request<GetExceptionsRequest>,
    ) -> Result<Response<GetExceptionsResponse>, Status> {
        let req = request.into_inner();

        // Get all batches via the query handler — in production this would
        // use a proper paginated query filtered by exception status
        let unmatched_query = query_types::GetUnmatchedRecordsQuery {
            settlement_batch_id: Uuid::nil(),
        };

        // Since we can't list all exceptions across batches directly,
        // return an empty list for now (the query infrastructure supports
        // per-batch unmatched records which covers exceptions)
        let _status_filter = if req.status.is_empty() {
            None
        } else {
            Some(req.status.as_str())
        };

        // Return empty response with pagination metadata
        Ok(Response::new(GetExceptionsResponse {
            exceptions: Vec::new(),
            pagination: Some(PaginationResponse {
                next_cursor: String::new(),
                has_more: false,
                as_of_unix_ms: Utc::now().timestamp_millis(),
            }),
        }))
    }

    async fn resolve_exception(
        &self,
        request: Request<ResolveExceptionRequest>,
    ) -> Result<Response<ResolveExceptionResponse>, Status> {
        let req = request.into_inner();
        let settlement_record_id = parse_uuid(&req.exception_id, "exception_id")?;
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;

        let cmd = commands::ResolveException {
            settlement_record_id,
            payment_intent_id,
            resolution: if req.resolution.is_empty() {
                "linked".to_string()
            } else {
                req.resolution
            },
        };

        match self.commands.resolve_exception(cmd).await {
            Ok(_event) => Ok(Response::new(ResolveExceptionResponse {
                status: "resolved".to_string(),
            })),
            Err(e) => Err(reconciliation_error_to_status(e)),
        }
    }

    async fn get_settlement_batches(
        &self,
        request: Request<GetBatchesRequest>,
    ) -> Result<Response<GetBatchesResponse>, Status> {
        let _req = request.into_inner();

        // The domain query handler doesn't have a batch listing query.
        // In production this would use a paginated query. For Phase 1,
        // return empty batch list with pagination.
        Ok(Response::new(GetBatchesResponse {
            batches: Vec::new(),
            pagination: Some(PaginationResponse {
                next_cursor: String::new(),
                has_more: false,
                as_of_unix_ms: Utc::now().timestamp_millis(),
            }),
        }))
    }

    async fn get_reconciliation_stats(
        &self,
        _request: Request<GetReconciliationStatsRequest>,
    ) -> Result<Response<GetReconciliationStatsResponse>, Status> {
        // Compute stats from domain data via query handler
        // For Phase 1, return basic stats
        Ok(Response::new(GetReconciliationStatsResponse {
            total_pending: 0,
            total_unmatched: 0,
            total_resolved: 0,
            match_rate: 0.0,
        }))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn reconciliation_error_to_status(e: ReconciliationError) -> Status {
    match e {
        ReconciliationError::NotFound(id) => {
            Status::not_found(format!("Settlement batch not found: {}", id))
        }
        ReconciliationError::Validation(msg) => Status::invalid_argument(msg),
        ReconciliationError::DuplicateBatch(checksum) => {
            Status::already_exists(format!("Duplicate batch: checksum {} already ingested", checksum))
        }
        ReconciliationError::InvariantViolation(msg) => Status::internal(msg),
        ReconciliationError::LedgerImbalance(id) => {
            Status::failed_precondition(format!("Ledger imbalance detected for transaction {}", id))
        }
    }
}

impl From<ReconciliationError> for Status {
    fn from(e: ReconciliationError) -> Self {
        reconciliation_error_to_status(e)
    }
}
