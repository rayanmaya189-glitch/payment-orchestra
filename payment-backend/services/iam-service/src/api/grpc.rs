//! gRPC service implementation for iam-service.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::queries::QueryHandler;
use crate::domain::{self, IamError};

use platform_proto::iam::iam_service_server::IamService;
use platform_proto::iam::*;

pub struct IamGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> IamGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> IamService for IamGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let req = request.into_inner();
        let ip_addr = req.ip_address.parse()
            .map_err(|_| Status::invalid_argument("Invalid IP address"))?;

        let cmd = commands::Authenticate {
            email: req.email,
            password: req.password,
            ip_address: ip_addr,
            user_agent: req.user_agent,
        };

        match self.commands.authenticate(cmd).await {
            Ok(result) => {
                Ok(Response::new(AuthenticateResponse {
                    principal_id: result.principal.id.to_string(),
                    access_token: result.access_token,
                    refresh_token: result.refresh_token,
                    expires_at_unix_ms: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp_millis(),
                    mfa_required: result.mfa_required,
                    mfa_method: result.mfa_method.unwrap_or_default(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn issue_token(
        &self,
        _request: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        Err(Status::unimplemented("Token refresh not yet implemented"))
    }

    async fn validate_permission(
        &self,
        _request: Request<ValidatePermissionRequest>,
    ) -> Result<Response<ValidatePermissionResponse>, Status> {
        // Simplified ABAC — always allow for now
        Ok(Response::new(ValidatePermissionResponse {
            allowed: true,
            requires_maker_checker: false,
            reason: "".into(),
        }))
    }

    async fn create_api_key(
        &self,
        request: Request<CreateApiKeyRequest>,
    ) -> Result<Response<CreateApiKeyResponse>, Status> {
        let req = request.into_inner();
        let principal_id = Uuid::parse_str(&req.principal_id)
            .map_err(|_| Status::invalid_argument("Invalid principal_id"))?;
        let expires_in_days = req.expires_in_days.parse().ok();

        let cmd = commands::CreateApiKey {
            principal_id,
            name: req.name,
            scopes: req.scopes,
            expires_in_days,
        };

        match self.commands.create_api_key(cmd).await {
            Ok(result) => {
                Ok(Response::new(CreateApiKeyResponse {
                    api_key_id: result.api_key.api_key_id.to_string(),
                    api_key_secret: result.api_key_secret,
                    name: result.api_key.name,
                    scopes: result.api_key.scopes,
                    status: result.api_key.status.as_str().to_string(),
                    expires_at_unix_ms: result.api_key.expires_at
                        .map(|t| t.timestamp_millis())
                        .unwrap_or(0),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn revoke_api_key(
        &self,
        request: Request<RevokeApiKeyRequest>,
    ) -> Result<Response<RevokeApiKeyResponse>, Status> {
        let req = request.into_inner();
        let api_key_id = Uuid::parse_str(&req.api_key_id)
            .map_err(|_| Status::invalid_argument("Invalid api_key_id"))?;
        let principal_id = Uuid::parse_str(&req.principal_id)
            .map_err(|_| Status::invalid_argument("Invalid principal_id"))?;

        let cmd = commands::RevokeApiKey {
            api_key_id,
            principal_id,
        };

        match self.commands.revoke_api_key(cmd).await {
            Ok(result) => {
                Ok(Response::new(RevokeApiKeyResponse {
                    revoked: result.revoked,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn list_api_keys(
        &self,
        request: Request<ListApiKeysRequest>,
    ) -> Result<Response<ListApiKeysResponse>, Status> {
        let req = request.into_inner();
        let principal_id = Uuid::parse_str(&req.principal_id)
            .map_err(|_| Status::invalid_argument("Invalid principal_id"))?;

        match self.queries.list_api_keys(principal_id).await {
            Ok(keys) => {
                let proto_keys: Vec<ApiKeyInfo> = keys.into_iter().map(|k| {
                    ApiKeyInfo {
                        api_key_id: k.api_key_id.to_string(),
                        name: k.name,
                        scopes: k.scopes,
                        status: k.status.as_str().to_string(),
                        created_at_unix_ms: k.created_at.timestamp_millis(),
                        expires_at_unix_ms: k.expires_at.map(|t| t.timestamp_millis()).unwrap_or(0),
                        last_used_at_unix_ms: k.last_used_at.map(|t| t.timestamp_millis()).unwrap_or(0),
                    }
                }).collect();

                Ok(Response::new(ListApiKeysResponse {
                    api_keys: proto_keys,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn submit_change(
        &self,
        request: Request<SubmitChangeRequest>,
    ) -> Result<Response<SubmitChangeResponse>, Status> {
        let req = request.into_inner();
        let maker_id = Uuid::parse_str(&req.maker_id)
            .map_err(|_| Status::invalid_argument("Invalid maker_id"))?;

        let cmd = commands::SubmitChange {
            change_type: req.change_type,
            maker_id,
            payload: req.payload,
            maker_note: if req.maker_note.is_empty() { None } else { Some(req.maker_note) },
        };

        match self.commands.submit_change(cmd).await {
            Ok(result) => {
                Ok(Response::new(SubmitChangeResponse {
                    change_id: result.change.change_id.to_string(),
                    status: result.change.status.as_str().to_string(),
                    expires_at_unix_ms: result.change.expires_at.timestamp_millis(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn review_change(
        &self,
        request: Request<ReviewChangeRequest>,
    ) -> Result<Response<ReviewChangeResponse>, Status> {
        let req = request.into_inner();
        let change_id = Uuid::parse_str(&req.change_id)
            .map_err(|_| Status::invalid_argument("Invalid change_id"))?;
        let checker_id = Uuid::parse_str(&req.checker_id)
            .map_err(|_| Status::invalid_argument("Invalid checker_id"))?;

        let cmd = commands::ReviewChange {
            change_id,
            checker_id,
            approved: req.approved,
            checker_note: if req.checker_note.is_empty() { None } else { Some(req.checker_note) },
        };

        match self.commands.review_change(cmd).await {
            Ok(result) => {
                Ok(Response::new(ReviewChangeResponse {
                    status: result.change.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn get_principal(
        &self,
        request: Request<GetPrincipalRequest>,
    ) -> Result<Response<GetPrincipalResponse>, Status> {
        let req = request.into_inner();
        let principal_id = Uuid::parse_str(&req.principal_id)
            .map_err(|_| Status::invalid_argument("Invalid principal_id"))?;

        match self.queries.get_principal(principal_id).await {
            Ok(Some(p)) => {
                Ok(Response::new(GetPrincipalResponse {
                    principal_id: p.id.to_string(),
                    principal_type: p.principal_type.as_str().to_string(),
                    email: p.email.unwrap_or_default(),
                    status: p.status.as_str().to_string(),
                    mfa_enrolled: p.mfa_enrolled,
                    mfa_method: p.mfa_method.as_ref().map(|m| m.as_str().to_string()).unwrap_or_default(),
                    created_at: Some(platform_proto::common::Timestamp {
                        unix_ms: p.created_at.timestamp_millis(),
                    }),
                    last_login_at: p.last_login_at.map(|t| platform_proto::common::Timestamp {
                        unix_ms: t.timestamp_millis(),
                    }),
                }))
            }
            Ok(None) => Err(Status::not_found("Principal not found")),
            Err(e) => Err(Status::from(e)),
        }
    }
}

// ─── Error Conversion ───────────────────────────────────────────────────────

impl From<IamError> for Status {
    fn from(e: IamError) -> Self {
        match e {
            IamError::PrincipalNotFound(_) => Status::not_found(e.to_string()),
            IamError::AuthorizationDenied(ref msg) => Status::permission_denied(msg.clone()),
            IamError::ChangeNotPending | IamError::SelfApprovalNotAllowed | IamError::ChangeExpired => {
                Status::failed_precondition(e.to_string())
            }
            IamError::InvalidRequest(ref msg) => Status::invalid_argument(msg.clone()),
            IamError::DuplicateApiKeyName(ref msg) => Status::already_exists(msg.clone()),
            IamError::Auth(domain::AuthError::InvalidCredentials) => {
                Status::unauthenticated("Invalid credentials")
            }
            IamError::Auth(domain::AuthError::AccountLocked(_)) => {
                Status::unauthenticated("Account locked")
            }
            IamError::Auth(domain::AuthError::AccountSuspended) => {
                Status::unauthenticated("Account suspended")
            }
            IamError::Auth(domain::AuthError::AccountDeleted) => {
                Status::unauthenticated("Account deleted")
            }
            IamError::Auth(domain::AuthError::MfaRequired) => {
                Status::unauthenticated("MFA required")
            }
            IamError::Auth(domain::AuthError::TokenExpired | domain::AuthError::InvalidToken) => {
                Status::unauthenticated(e.to_string())
            }
        }
    }
}
