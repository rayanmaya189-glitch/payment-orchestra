//! gRPC service implementation for dispute-service (BC-10).
//! Translates between protobuf types and domain types for chargeback lifecycle.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{
    CommandHandler, RecordChargebackCommand, SubmitRepresentmentCommand,
    ResolveChargebackCommand,
};
use crate::domain::{
    self, ChargebackOutcome, ChargebackStatus, DisputeError, RepresentmentEvidence,
};

use platform_proto::common::{Money as ProtoMoney, Timestamp, PaginationResponse};
use platform_proto::dispute::dispute_service_server::DisputeService;
use platform_proto::dispute::*;

pub struct DisputeGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> DisputeGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> DisputeService for DisputeGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: crate::queries::QueryHandler + Send + Sync + 'static,
{
    async fn record_chargeback(
        &self,
        request: Request<RecordChargebackRequest>,
    ) -> Result<Response<RecordChargebackResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let payment_intent_id = parse_uuid(&req.payment_intent_id, "payment_intent_id")?;
        let acquirer_link_id = parse_uuid(&req.acquirer_link_id, "acquirer_link_id")?;

        let amount = req
            .amount
            .ok_or_else(|| Status::invalid_argument("amount is required"))?;

        let cmd = RecordChargebackCommand {
            operator_id,
            payment_intent_id,
            acquirer_link_id,
            reason_code: req.reason_code,
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency_code,
            is_captured: true, // caller is responsible for ensuring captured status
        };

        match self.commands.record_chargeback(cmd).await {
            Ok(case) => {
                Ok(Response::new(RecordChargebackResponse {
                    chargeback_id: case.chargeback_id.to_string(),
                    status: case.status.to_string(),
                    deadline: Some(Timestamp {
                        unix_ms: case.representment_deadline.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(dispute_error_to_status(e)),
        }
    }

    async fn submit_representment(
        &self,
        request: Request<SubmitRepresentmentRequest>,
    ) -> Result<Response<SubmitRepresentmentResponse>, Status> {
        let req = request.into_inner();
        let chargeback_id = parse_uuid(&req.chargeback_id, "chargeback_id")?;

        let evidence_doc_ids: Vec<Uuid> = req
            .evidence_document_ids
            .iter()
            .map(|id| {
                Uuid::parse_str(id).map_err(|_| {
                    Status::invalid_argument(format!("Invalid evidence document ID: {}", id))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let evidence = RepresentmentEvidence {
            transaction_receipt: None,
            delivery_confirmation: None,
            customer_communication: None,
            cardholder_agreement: None,
            refund_policy: None,
            description: req.narrative,
            supporting_documents: evidence_doc_ids,
        };

        let cmd = SubmitRepresentmentCommand {
            chargeback_id,
            evidence,
        };

        match self.commands.submit_representment(cmd).await {
            Ok(case) => {
                // Find the latest submission timestamp
                let submitted_at = case
                    .submissions
                    .last()
                    .map(|s| s.submitted_at)
                    .unwrap_or_else(chrono::Utc::now);

                Ok(Response::new(SubmitRepresentmentResponse {
                    status: case.status.to_string(),
                    submitted_at: Some(Timestamp {
                        unix_ms: submitted_at.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(dispute_error_to_status(e)),
        }
    }

    async fn resolve_chargeback(
        &self,
        request: Request<ResolveChargebackRequest>,
    ) -> Result<Response<ResolveChargebackResponse>, Status> {
        let req = request.into_inner();
        let chargeback_id = parse_uuid(&req.chargeback_id, "chargeback_id")?;

        let outcome = match req.resolution.as_str() {
            "won" => ChargebackOutcome::Won,
            "lost" => ChargebackOutcome::Lost,
            "accepted" => ChargebackOutcome::Accepted,
            "escalated" => ChargebackOutcome::Escalated,
            other => {
                return Err(Status::invalid_argument(format!(
                    "Invalid resolution '{}': expected 'won', 'lost', 'accepted', or 'escalated'",
                    other
                )));
            }
        };

        let resolution_note = if req.notes.is_empty() {
            None
        } else {
            Some(req.notes)
        };

        let cmd = ResolveChargebackCommand {
            chargeback_id,
            outcome,
            resolution_note,
        };

        match self.commands.resolve_chargeback(cmd).await {
            Ok(_) => Ok(Response::new(ResolveChargebackResponse {
                status: "resolved".to_string(),
            })),
            Err(e) => Err(dispute_error_to_status(e)),
        }
    }

    async fn get_chargeback_case(
        &self,
        request: Request<GetChargebackCaseRequest>,
    ) -> Result<Response<ChargebackCaseView>, Status> {
        let req = request.into_inner();
        let chargeback_id = parse_uuid(&req.chargeback_id, "chargeback_id")?;

        match self.queries.get_chargeback(chargeback_id).await {
            Ok(case) => Ok(Response::new(chargeback_to_view(case))),
            Err(e) => Err(dispute_error_to_status(e)),
        }
    }

    async fn list_chargebacks(
        &self,
        request: Request<ListChargebacksRequest>,
    ) -> Result<Response<ListChargebacksResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        let status_filter = if req.status_filter.is_empty() {
            None
        } else {
            Some(req.status_filter.parse::<ChargebackStatus>().map_err(|_| {
                Status::invalid_argument(format!("Invalid status filter: {}", req.status_filter))
            })?)
        };

        let page_limit = req.pagination.as_ref().map_or(50, |p| {
            if p.limit > 0 && p.limit <= 100 { p.limit as usize } else { 50 }
        });
        let cursor = req.pagination.as_ref().and_then(|p| {
            if p.cursor.is_empty() { None } else { Some(p.cursor.clone()) }
        });

        match self.queries.find_by_operator(operator_id).await {
            Ok(all_cases) => {
                let filtered: Vec<domain::ChargebackCase> = all_cases
                    .into_iter()
                    .filter(|case| {
                        if let Some(ref filter) = status_filter {
                            &case.status == filter
                        } else {
                            true
                        }
                    })
                    .skip_while(|case| {
                        if let Some(ref c) = cursor {
                            case.chargeback_id.to_string() != *c
                        } else {
                            false
                        }
                    })
                    .take(page_limit)
                    .collect();

                let has_more = filtered.len() >= page_limit;
                let next_cursor = filtered.last().map(|case| case.chargeback_id.to_string());
                let views: Vec<ChargebackCaseView> =
                    filtered.into_iter().map(chargeback_to_view).collect();

                Ok(Response::new(ListChargebacksResponse {
                    chargebacks: views,
                    pagination: Some(PaginationResponse {
                        next_cursor: next_cursor.unwrap_or_default(),
                        has_more,
                        as_of_unix_ms: chrono::Utc::now().timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(dispute_error_to_status(e)),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn chargeback_to_view(case: domain::ChargebackCase) -> ChargebackCaseView {
    let evidence_ids: Vec<String> = case
        .submissions
        .last()
        .map(|s| {
            s.evidence
                .supporting_documents
                .iter()
                .map(|id| id.to_string())
                .collect()
        })
        .unwrap_or_default();

    ChargebackCaseView {
        chargeback_id: case.chargeback_id.to_string(),
        payment_intent_id: case.payment_intent_id.to_string(),
        operator_id: case.operator_id.to_string(),
        status: case.status.to_string(),
        amount: Some(ProtoMoney {
            amount_minor_units: case.amount_minor_units,
            currency_code: case.currency,
        }),
        reason_code: case.reason_code,
        reason_description: String::new(),
        acquirer_link_id: case.acquirer_link_id.to_string(),
        received_at: Some(Timestamp {
            unix_ms: case.received_at.timestamp_millis(),
        }),
        deadline: Some(Timestamp {
            unix_ms: case.representment_deadline.timestamp_millis(),
        }),
        representment_submitted_at: case
            .submissions
            .last()
            .map(|s| Timestamp {
                unix_ms: s.submitted_at.timestamp_millis(),
            }),
        resolved_at: case.resolved_at.map(|dt| Timestamp {
            unix_ms: dt.timestamp_millis(),
        }),
        resolution: case
            .outcome
            .map(|o| match o {
                ChargebackOutcome::Won => "won".to_string(),
                ChargebackOutcome::Lost => "lost".to_string(),
                ChargebackOutcome::Accepted => "accepted".to_string(),
                ChargebackOutcome::Escalated => "escalated".to_string(),
            })
            .unwrap_or_default(),
        evidence_document_ids: evidence_ids,
    }
}

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

fn dispute_error_to_status(e: DisputeError) -> Status {
    match e {
        DisputeError::NotFound(id) => {
            Status::not_found(format!("Chargeback not found: {}", id))
        }
        DisputeError::AlreadyResolved => {
            Status::failed_precondition("Chargeback already resolved")
        }
        DisputeError::PaymentIntentNotCaptured => {
            Status::failed_precondition("Payment intent must be in captured state")
        }
        DisputeError::InvalidTransition => {
            Status::failed_precondition("Invalid chargeback status transition")
        }
        DisputeError::InvalidAmount => {
            Status::invalid_argument("Invalid chargeback amount")
        }
        DisputeError::InvalidRepresentmentEvidence => {
            Status::invalid_argument("Invalid representment evidence: missing required fields")
        }
        DisputeError::RepresentmentDeadlinePassed => {
            Status::failed_precondition("Representment deadline has passed")
        }
        DisputeError::DatabaseError(msg) => {
            Status::internal(format!("Database error: {}", msg))
        }
    }
}

impl From<DisputeError> for Status {
    fn from(e: DisputeError) -> Self {
        dispute_error_to_status(e)
    }
}
