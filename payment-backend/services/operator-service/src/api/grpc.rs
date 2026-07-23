//! gRPC service implementation for operator-service.
//! Translates between protobuf types and domain types.
//! Types come from platform_proto which compiles all .proto files.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{RegisterOperator, VerifyEmail, UpdateOperatorStatus};
use crate::domain::OperatorStatus;

// Import generated types from platform-proto crate.
// Module structure matches proto package names (e.g., `package operator.v1` → `operator` module)
use platform_proto::operator::operator_service_server::OperatorService;
use platform_proto::operator::*;
use platform_proto::common::Timestamp;

pub struct OperatorGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> OperatorGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> OperatorService for OperatorGrpcService<C, Q>
where
    C: crate::commands::CommandHandler + Send + Sync + 'static,
    Q: crate::queries::QueryHandler + Send + Sync + 'static,
{
    async fn register_operator(
        &self,
        request: Request<RegisterOperatorRequest>,
    ) -> Result<Response<RegisterOperatorResponse>, Status> {
        let req = request.into_inner();
        let cmd = RegisterOperator {
            legal_name: req.legal_name,
            trade_license_no: req.trade_license_no,
            country: req.country,
            email: req.email,
        };

        match self.commands.register(cmd).await {
            Ok(result) => {
                Ok(Response::new(RegisterOperatorResponse {
                    operator_id: result.operator.id.to_string(),
                    subdomain: result.operator.subdomain,
                    status: result.operator.status.as_str().to_string(),
                    created_at: Some(Timestamp {
                        unix_ms: result.operator.created_at.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn verify_email(
        &self,
        request: Request<VerifyEmailRequest>,
    ) -> Result<Response<VerifyEmailResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;

        let cmd = VerifyEmail {
            operator_id,
            verification_token: req.verification_token,
        };

        match self.commands.verify_email(cmd).await {
            Ok(result) => {
                Ok(Response::new(VerifyEmailResponse {
                    status: result.operator.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn update_operator_status(
        &self,
        request: Request<UpdateOperatorStatusRequest>,
    ) -> Result<Response<UpdateOperatorStatusResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;
        let new_status = OperatorStatus::from_str(&req.new_status)
            .ok_or_else(|| Status::invalid_argument(format!("Invalid status: {}", req.new_status)))?;
        let changed_by = Uuid::parse_str(&req.changed_by)
            .map_err(|_| Status::invalid_argument("Invalid changed_by"))?;

        let cmd = UpdateOperatorStatus {
            operator_id,
            new_status,
            reason: req.reason,
            changed_by,
        };

        match self.commands.update_status(cmd).await {
            Ok(result) => {
                Ok(Response::new(UpdateOperatorStatusResponse {
                    status: result.operator.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn get_operator(
        &self,
        request: Request<GetOperatorRequest>,
    ) -> Result<Response<GetOperatorResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;

        match self.queries.get_operator(operator_id).await {
            Ok(Some(op)) => {
                Ok(Response::new(GetOperatorResponse {
                    operator_id: op.id.to_string(),
                    legal_name: op.legal_name,
                    trade_license_no: op.trade_license_no,
                    country: op.country,
                    status: op.status.as_str().to_string(),
                    subdomain: op.subdomain,
                    created_at: Some(Timestamp {
                        unix_ms: op.created_at.timestamp_millis(),
                    }),
                    provisioned_at: op.provisioned_at.map(|t| Timestamp {
                        unix_ms: t.timestamp_millis(),
                    }),
                }))
            }
            Ok(None) => Err(Status::not_found("Operator not found")),
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn list_operators(
        &self,
        request: Request<ListOperatorsRequest>,
    ) -> Result<Response<ListOperatorsResponse>, Status> {
        let req = request.into_inner();
        let status_filter = if req.status_filter.is_empty() { None } else { Some(req.status_filter.as_str()) };

        match self.queries.list_operators(status_filter).await {
            Ok(operators) => {
                let proto_operators: Vec<GetOperatorResponse> = operators.into_iter().map(|op| {
                    GetOperatorResponse {
                        operator_id: op.id.to_string(),
                        legal_name: op.legal_name,
                        trade_license_no: op.trade_license_no,
                        country: op.country,
                        status: op.status.as_str().to_string(),
                        subdomain: op.subdomain,
                        created_at: Some(Timestamp {
                            unix_ms: op.created_at.timestamp_millis(),
                        }),
                        provisioned_at: op.provisioned_at.map(|t| Timestamp {
                            unix_ms: t.timestamp_millis(),
                        }),
                    }
                }).collect();

                Ok(Response::new(ListOperatorsResponse {
                    operators: proto_operators,
                    pagination: None,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }
}

// ─── Error Conversion ───────────────────────────────────────────────────────

impl From<crate::domain::OperatorError> for Status {
    fn from(e: crate::domain::OperatorError) -> Self {
        match e {
            crate::domain::OperatorError::NotFound(id) => {
                Status::not_found(format!("Operator not found: {}", id))
            }
            crate::domain::OperatorError::DuplicateTradeLicense(ref l) => {
                Status::already_exists(format!("Duplicate trade license: {}", l))
            }
            crate::domain::OperatorError::DuplicateSubdomain(ref s) => {
                Status::already_exists(format!("Duplicate subdomain: {}", s))
            }
            crate::domain::OperatorError::InvalidTradeLicenseFormat(ref msg) => {
                Status::invalid_argument(format!("Invalid trade license: {}", msg))
            }
            crate::domain::OperatorError::InvalidStatusTransition { from, to } => {
                Status::failed_precondition(format!("Cannot transition from {} to {}", from, to))
            }
            crate::domain::OperatorError::EmailVerificationFailed(ref msg) => {
                Status::failed_precondition(format!("Email verification failed: {}", msg))
            }
            crate::domain::OperatorError::Suspended => {
                Status::failed_precondition("Operator suspended")
            }            crate::domain::OperatorError::KybNotApproved => {
                Status::failed_precondition("KYB not approved")
            }
            crate::domain::OperatorError::DatabaseError(ref msg) => {
                Status::internal(format!("Database error: {}", msg))
            }
        }
    }
}


