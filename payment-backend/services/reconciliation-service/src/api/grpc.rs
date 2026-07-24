#![allow(clippy::too_many_lines)]
//! gRPC service implementation for reconciliation-service (BC-09).
//! Translates between protobuf types and domain types for settlement matching.

use chrono::Utc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::domain::*;
use crate::repository::SettlementBatchRepository;

use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationResponse};
use platform_proto::reconciliation::reconciliation_service_server::ReconciliationService;
use platform_proto::reconciliation::*;
use platform_proto::reconciliation::SettlementBatch as ProtoSettlementBatch;

pub struct ReconciliationGrpcService<C, Q, R> {
    commands: C,
    _queries: Q,
    repo: R,
}

impl<C, Q, R> ReconciliationGrpcService<C, Q, R> {
    pub fn new(commands: C, queries: Q, repo: R) -> Self {
        Self { commands, _queries: queries, repo }
    }
}

#[tonic::async_trait]
impl<C, Q, R> ReconciliationService for ReconciliationGrpcService<C, Q, R>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: Send + Sync + 'static,
    R: SettlementBatchRepository + Send + Sync + 'static,
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

        match self.repo.list_all_batches().await {
            Ok(batches) => {
                let mut exceptions = Vec::new();

                for batch in &batches {
                    for record in &batch.records {
                        // A record is an exception if it's unmatched or has an amount mismatch
                        let is_exception = match &record.match_outcome {
                            Some(SettlementMatchOutcome::Unmatched)
                            | Some(SettlementMatchOutcome::AmountMismatch)
                            | Some(SettlementMatchOutcome::DuplicateReference) => true,
                            None => true, // Not yet processed == exception
                            _ => false,
                        };

                        if !is_exception {
                            continue;
                        }

                        // Apply status filter if specified
                        if !req.status.is_empty() {
                            let status_str = match &record.match_outcome {
                                Some(SettlementMatchOutcome::Unmatched) => "unmatched",
                                Some(SettlementMatchOutcome::AmountMismatch) => "amount_mismatch",
                                Some(SettlementMatchOutcome::DuplicateReference) => {
                                    "duplicate_reference"
                                }
                                None => "unmatched",
                                _ => continue,
                            };
                            if status_str != req.status {
                                continue;
                            }
                        }

                        exceptions.push(ReconciliationException {
                            exception_id: record.record_id.to_string(),
                            settlement_record_id: record.record_id.to_string(),
                            payment_intent_id: record
                                .matched_payment_intent_id
                                .map(|id| id.to_string())
                                .unwrap_or_default(),
                            amount: Some(ProtoMoney {
                                amount_minor_units: record.amount_minor.abs(),
                                currency_code: record.currency.clone(),
                            }),
                            status: match &record.match_outcome {
                                Some(SettlementMatchOutcome::Unmatched) => "unmatched".into(),
                                Some(SettlementMatchOutcome::AmountMismatch) => {
                                    "amount_mismatch".into()
                                }
                                Some(SettlementMatchOutcome::DuplicateReference) => {
                                    "duplicate_reference".into()
                                }
                                None => "pending".into(),
                                _ => "resolved".into(),
                            },
                            classification: "unmatched".into(),
                            detected_at: Some(Timestamp {
                                unix_ms: batch.ingested_at.timestamp_millis(),
                            }),
                        });
                    }
                }

                // Sort by most recent first
                exceptions.reverse();

                Ok(Response::new(GetExceptionsResponse {
                    exceptions,
                    pagination: Some(PaginationResponse {
                        next_cursor: String::new(),
                        has_more: false,
                        as_of_unix_ms: Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(reconciliation_error_to_status(e)),
        }
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
        let req = request.into_inner();

        match self.repo.list_all_batches().await {
            Ok(all_batches) => {
                let filtered: Vec<crate::domain::SettlementBatch> = if req.status_filter.is_empty() {
                    all_batches
                } else {
                    all_batches
                        .into_iter()
                        .filter(|b| b.status.to_string() == req.status_filter)
                        .collect()
                };

                let batches: Vec<ProtoSettlementBatch> = filtered
                    .into_iter()
                    .map(|b| ProtoSettlementBatch {
                        batch_id: b.settlement_batch_id.to_string(),
                        acquirer_link_id: b.acquirer_link_id.to_string(),
                        total_records: b.total_records,
                        matched_records: b.matched_count,
                        exception_count: b.unmatched_count,
                        settlement_date: None,
                        ingested_at: Some(Timestamp {
                            unix_ms: b.ingested_at.timestamp_millis(),
                        }),
                        reconciled_at: b.processed_at.map(|t| Timestamp {
                            unix_ms: t.timestamp_millis(),
                        }),
                        status: b.status.to_string(),
                    })
                    .collect();

                Ok(Response::new(GetBatchesResponse {
                    batches,
                    pagination: Some(PaginationResponse {
                        next_cursor: String::new(),
                        has_more: false,
                        as_of_unix_ms: Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(reconciliation_error_to_status(e)),
        }
    }

    async fn get_reconciliation_stats(
        &self,
        _request: Request<GetReconciliationStatsRequest>,
    ) -> Result<Response<GetReconciliationStatsResponse>, Status> {
        match self.repo.list_all_batches().await {
            Ok(batches) => {
                let mut total_pending: i32 = 0;
                let mut total_unmatched: i32 = 0;
                let mut total_resolved: i32 = 0;
                let mut total_records: i32 = 0;
                let mut matched_records: i32 = 0;

                for batch in &batches {
                    match &batch.status {
                        BatchStatus::Ingesting => total_pending += 1,
                        BatchStatus::Processed => {
                            total_resolved += 1;
                            total_records += batch.total_records;
                            matched_records += batch.matched_count;
                        }
                        BatchStatus::Quarantined => {
                            total_unmatched += 1;
                        }
                    }
                }

                let match_rate = if total_records > 0 {
                    (matched_records as f64 / total_records as f64) * 100.0
                } else {
                    0.0
                };

                Ok(Response::new(GetReconciliationStatsResponse {
                    total_pending,
                    total_unmatched,
                    total_resolved,
                    match_rate: (match_rate * 100.0).round() / 100.0,
                }))
            }
            Err(e) => Err(reconciliation_error_to_status(e)),
        }
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
        ReconciliationError::DatabaseError(msg) => {
            Status::internal(format!("Database error: {}", msg))
        }
    }
}

impl From<ReconciliationError> for Status {
    fn from(e: ReconciliationError) -> Self {
        reconciliation_error_to_status(e)
    }
}
