//! gRPC service implementation for compliance-service.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler, AmlAlertDecision};
use crate::queries::QueryHandler;
use crate::domain::ComplianceError;

use platform_proto::compliance::compliance_service_server::ComplianceService;
use platform_proto::compliance::*;

pub struct ComplianceGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> ComplianceGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> ComplianceService for ComplianceGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn submit_kyb_evidence(
        &self,
        request: Request<SubmitKybEvidenceRequest>,
    ) -> Result<Response<SubmitKybEvidenceResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;
        let submitted_by = Uuid::parse_str(&req.submitted_by)
            .map_err(|_| Status::invalid_argument("Invalid submitted_by"))?;
        let document_ids: Vec<Uuid> = req.document_ids.iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();

        let cmd = commands::SubmitKybEvidence {
            operator_id,
            document_ids,
            submitted_by,
        };

        match self.commands.submit_kyb_evidence(cmd).await {
            Ok(result) => {
                Ok(Response::new(SubmitKybEvidenceResponse {
                    kyb_case_id: result.kyb_case.kyb_case_id.to_string(),
                    status: result.kyb_case.status.as_str().to_string(),
                    submitted_at: Some(platform_proto::common::Timestamp {
                        unix_ms: result.kyb_case.submitted_at.timestamp_millis(),
                    }),
                    document_count: result.kyb_case.document_ids.len() as i32,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn review_kyb_case(
        &self,
        request: Request<ReviewKybCaseRequest>,
    ) -> Result<Response<ReviewKybCaseResponse>, Status> {
        let req = request.into_inner();
        let kyb_case_id = Uuid::parse_str(&req.kyb_case_id)
            .map_err(|_| Status::invalid_argument("Invalid kyb_case_id"))?;
        let reviewed_by = Uuid::parse_str(&req.reviewed_by)
            .map_err(|_| Status::invalid_argument("Invalid reviewed_by"))?;

        let cmd = commands::ReviewKybCase {
            kyb_case_id,
            approved: req.approved,
            reason: if req.reason.is_empty() { None } else { Some(req.reason) },
            reviewed_by,
        };

        match self.commands.review_kyb_case(cmd).await {
            Ok(result) => {
                Ok(Response::new(ReviewKybCaseResponse {
                    status: result.kyb_case.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn get_kyb_case(
        &self,
        request: Request<GetKybCaseRequest>,
    ) -> Result<Response<GetKybCaseResponse>, Status> {
        let req = request.into_inner();
        let kyb_case_id = Uuid::parse_str(&req.kyb_case_id)
            .map_err(|_| Status::invalid_argument("Invalid kyb_case_id"))?;

        match self.queries.get_kyb_case(kyb_case_id).await {
            Ok(Some(kase)) => {
                Ok(Response::new(GetKybCaseResponse {
                    kyb_case_id: kase.kyb_case_id.to_string(),
                    operator_id: kase.operator_id.to_string(),
                    status: kase.status.as_str().to_string(),
                    document_ids: kase.document_ids.iter().map(|id| id.to_string()).collect(),
                    rejection_reason: kase.rejection_reason.unwrap_or_default(),
                    submitted_at: Some(platform_proto::common::Timestamp {
                        unix_ms: kase.submitted_at.timestamp_millis(),
                    }),
                    resolved_at: kase.resolved_at.map(|t| platform_proto::common::Timestamp {
                        unix_ms: t.timestamp_millis(),
                    }),
                }))
            }
            Ok(None) => Err(Status::not_found("KYB case not found")),
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn list_pending_kyb_cases(
        &self,
        _request: Request<ListPendingKybCasesRequest>,
    ) -> Result<Response<ListPendingKybCasesResponse>, Status> {
        match self.queries.list_pending_kyb_cases().await {
            Ok(cases) => {
                let proto_cases: Vec<GetKybCaseResponse> = cases.into_iter().map(|kase| {
                    GetKybCaseResponse {
                        kyb_case_id: kase.kyb_case_id.to_string(),
                        operator_id: kase.operator_id.to_string(),
                        status: kase.status.as_str().to_string(),
                        document_ids: kase.document_ids.iter().map(|id| id.to_string()).collect(),
                        rejection_reason: kase.rejection_reason.unwrap_or_default(),
                        submitted_at: Some(platform_proto::common::Timestamp {
                            unix_ms: kase.submitted_at.timestamp_millis(),
                        }),
                        resolved_at: kase.resolved_at.map(|t| platform_proto::common::Timestamp {
                            unix_ms: t.timestamp_millis(),
                        }),
                    }
                }).collect();

                Ok(Response::new(ListPendingKybCasesResponse {
                    cases: proto_cases,
                    pagination: None,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn scan_transaction(
        &self,
        request: Request<ScanTransactionRequest>,
    ) -> Result<Response<ScanTransactionResponse>, Status> {
        let req = request.into_inner();
        let transaction_id = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction_id"))?;
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;

        let cmd = commands::ScanTransaction {
            transaction_id,
            operator_id,
            amount_minor_units: req.amount_minor_units,
            payment_method_id: if req.payment_method_id.is_empty() { None } else { Some(req.payment_method_id) },
        };

        match self.commands.scan_transaction(cmd).await {
            Ok(result) => {
                let proto_alerts: Vec<AmlAlertProto> = result.alerts.into_iter().map(|a| {
                    AmlAlertProto {
                        alert_id: a.alert_id.to_string(),
                        alert_type: a.alert_type.as_str().to_string(),
                        severity: a.severity.as_str().to_string(),
                        rule_id: a.rule_id,
                        details: a.details,
                        created_at_unix_ms: a.created_at.timestamp_millis(),
                    }
                }).collect();

                Ok(Response::new(ScanTransactionResponse {
                    alerts: proto_alerts,
                    blocked: result.blocked,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn list_aml_alerts(
        &self,
        request: Request<ListAmlAlertsRequest>,
    ) -> Result<Response<ListAmlAlertsResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;
        let status_filter = if req.status_filter.is_empty() { None } else { Some(req.status_filter.as_str()) };

        match self.queries.list_aml_alerts(operator_id, status_filter).await {
            Ok(alerts) => {
                let proto_alerts: Vec<AmlAlertProto> = alerts.into_iter().map(|a| {
                    AmlAlertProto {
                        alert_id: a.alert_id.to_string(),
                        alert_type: a.alert_type.as_str().to_string(),
                        severity: a.severity.as_str().to_string(),
                        rule_id: a.rule_id,
                        details: a.details,
                        created_at_unix_ms: a.created_at.timestamp_millis(),
                    }
                }).collect();

                Ok(Response::new(ListAmlAlertsResponse {
                    alerts: proto_alerts,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn review_aml_alert(
        &self,
        request: Request<ReviewAmlAlertRequest>,
    ) -> Result<Response<ReviewAmlAlertResponse>, Status> {
        let req = request.into_inner();
        let alert_id = Uuid::parse_str(&req.alert_id)
            .map_err(|_| Status::invalid_argument("Invalid alert_id"))?;
        let reviewer_id = Uuid::parse_str(&req.reviewer_id)
            .map_err(|_| Status::invalid_argument("Invalid reviewer_id"))?;

        let decision = match req.decision.as_str() {
            "escalated" => AmlAlertDecision::Escalated,
            "closed_false_positive" => AmlAlertDecision::ClosedFalsePositive,
            _ => return Err(Status::invalid_argument("Invalid decision")),
        };

        let cmd = commands::ReviewAmlAlert {
            alert_id,
            reviewer_id,
            decision,
            notes: if req.notes.is_empty() { None } else { Some(req.notes) },
        };

        match self.commands.review_aml_alert(cmd).await {
            Ok(result) => {
                Ok(Response::new(ReviewAmlAlertResponse {
                    status: result.alert.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }
}

// ─── Error Conversion ───────────────────────────────────────────────────────

impl From<ComplianceError> for Status {
    fn from(e: ComplianceError) -> Self {
        match e {
            ComplianceError::KybCaseNotFound(_) | ComplianceError::AmlAlertNotFound(_) => {
                Status::not_found(e.to_string())
            }
            ComplianceError::KybCaseAlreadyResolved(_) | ComplianceError::AmlAlertAlreadyReviewed(_) => {
                Status::failed_precondition(e.to_string())
            }
            ComplianceError::KybNoDocuments => {
                Status::invalid_argument("At least one document required")
            }
            ComplianceError::InvalidRequest(ref msg) => {
                Status::invalid_argument(msg.clone())
            }
            ComplianceError::SarGenerationFailed(ref msg) => {
                Status::internal(msg.clone())
            }
            ComplianceError::PartnerApiUnavailable(ref msg) => {
                Status::unavailable(msg.clone())
            }
        }
    }
}
