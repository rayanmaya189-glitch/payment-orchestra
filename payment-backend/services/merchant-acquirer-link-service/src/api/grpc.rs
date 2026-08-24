//! gRPC service implementation for merchant-acquirer-link service.

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::commands::{self, CommandHandler};
use crate::queries::QueryHandler;
use crate::domain::{LinkEnvironment, LinkError};

use platform_proto::connector::merchant_acquirer_link_service_server::MerchantAcquirerLinkService;
use platform_proto::connector::*;

pub struct LinkGrpcService<C, Q> {
    commands: C,
    queries: Q,
}

impl<C, Q> LinkGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q) -> Self {
        Self { commands, queries }
    }
}

#[tonic::async_trait]
impl<C, Q> MerchantAcquirerLinkService for LinkGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn create_link(
        &self,
        request: Request<CreateLinkRequest>,
    ) -> Result<Response<CreateLinkResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;
        let environment = match req.environment.as_str() {
            "sandbox" => LinkEnvironment::Sandbox,
            "production" => LinkEnvironment::Production,
            _ => return Err(Status::invalid_argument("Invalid environment")),
        };

        let cmd = commands::CreateLink {
            operator_id,
            connector_id: req.connector_id,
            display_name: req.display_name,
            environment,
            credentials: req.credentials,
        };

        match self.commands.create_link(cmd).await {
            Ok(result) => {
                Ok(Response::new(CreateLinkResponse {
                    link_id: result.link.link_id.to_string(),
                    connector_id: result.link.connector_id,
                    display_name: result.link.display_name,
                    environment: result.link.environment.as_str().to_string(),
                    status: result.link.status.as_str().to_string(),
                    health_status: result.link.health_status.as_str().to_string(),
                    created_at: Some(platform_proto::common::Timestamp {
                        unix_ms: result.link.created_at.timestamp_millis(),
                    }),
                    test_connection_url: format!("/v1/merchant-links/{}/test", result.link.link_id),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn test_connection(
        &self,
        request: Request<TestConnectionRequest>,
    ) -> Result<Response<TestConnectionResponse>, Status> {
        let req = request.into_inner();
        let link_id = Uuid::parse_str(&req.link_id)
            .map_err(|_| Status::invalid_argument("Invalid link_id"))?;

        match self.commands.test_connection(commands::TestConnection { link_id }).await {
            Ok(result) => {
                Ok(Response::new(TestConnectionResponse {
                    link_id: result.link.link_id.to_string(),
                    status: result.link.status.as_str().to_string(),
                    health_status: result.link.health_status.as_str().to_string(),
                    success: result.success,
                    latency_ms: result.latency_ms as i32,
                    error_message: result.error_message.unwrap_or_default(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn rotate_credentials(
        &self,
        request: Request<RotateCredentialsRequest>,
    ) -> Result<Response<RotateCredentialsResponse>, Status> {
        let req = request.into_inner();
        let link_id = Uuid::parse_str(&req.link_id)
            .map_err(|_| Status::invalid_argument("Invalid link_id"))?;

        let cmd = commands::RotateCredentials {
            link_id,
            new_credentials: req.new_credentials,
            rotate_immediately: req.rotate_immediately,
        };

        match self.commands.rotate_credentials(cmd).await {
            Ok(result) => {
                Ok(Response::new(RotateCredentialsResponse {
                    link_id: result.link.link_id.to_string(),
                    status: result.link.status.as_str().to_string(),
                    old_credentials_retained: result.old_credentials_retained,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn disable_link(
        &self,
        request: Request<DisableLinkRequest>,
    ) -> Result<Response<DisableLinkResponse>, Status> {
        let req = request.into_inner();
        let link_id = Uuid::parse_str(&req.link_id)
            .map_err(|_| Status::invalid_argument("Invalid link_id"))?;

        let cmd = commands::DisableLink {
            link_id,
            reason: req.reason,
        };

        match self.commands.disable_link(cmd).await {
            Ok(result) => {
                Ok(Response::new(DisableLinkResponse {
                    status: result.link.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn enable_link(
        &self,
        request: Request<EnableLinkRequest>,
    ) -> Result<Response<EnableLinkResponse>, Status> {
        let req = request.into_inner();
        let link_id = Uuid::parse_str(&req.link_id)
            .map_err(|_| Status::invalid_argument("Invalid link_id"))?;

        match self.commands.enable_link(commands::EnableLink { link_id }).await {
            Ok(result) => {
                Ok(Response::new(EnableLinkResponse {
                    status: result.link.status.as_str().to_string(),
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn update_link_metadata(
        &self,
        request: Request<UpdateLinkMetadataRequest>,
    ) -> Result<Response<UpdateLinkMetadataResponse>, Status> {
        let req = request.into_inner();
        let link_id = Uuid::parse_str(&req.link_id)
            .map_err(|_| Status::invalid_argument("Invalid link_id"))?;

        let cmd = commands::UpdateMetadata {
            link_id,
            display_name: if req.display_name.is_empty() { None } else { Some(req.display_name) },
        };

        match self.commands.update_metadata(cmd).await {
            Ok(result) => {
                Ok(Response::new(UpdateLinkMetadataResponse {
                    link_id: result.link.link_id.to_string(),
                    display_name: result.link.display_name,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn get_link(
        &self,
        request: Request<GetLinkRequest>,
    ) -> Result<Response<GetLinkResponse>, Status> {
        let req = request.into_inner();
        let link_id = Uuid::parse_str(&req.link_id)
            .map_err(|_| Status::invalid_argument("Invalid link_id"))?;

        match self.queries.get_link(link_id).await {
            Ok(Some(link)) => {
                Ok(Response::new(GetLinkResponse {
                    link_id: link.link_id.to_string(),
                    operator_id: link.operator_id.to_string(),
                    connector_id: link.connector_id,
                    display_name: link.display_name,
                    environment: link.environment.as_str().to_string(),
                    status: link.status.as_str().to_string(),
                    health_status: link.health_status.as_str().to_string(),
                    created_at: Some(platform_proto::common::Timestamp {
                        unix_ms: link.created_at.timestamp_millis(),
                    }),
                    last_tested_at: link.last_tested_at.map(|t| platform_proto::common::Timestamp {
                        unix_ms: t.timestamp_millis(),
                    }),
                    last_healthy_at: link.last_healthy_at.map(|t| platform_proto::common::Timestamp {
                        unix_ms: t.timestamp_millis(),
                    }),
                    credentials_expires_at: link.credentials_expires_at.map(|t| platform_proto::common::Timestamp {
                        unix_ms: t.timestamp_millis(),
                    }),
                }))
            }
            Ok(None) => Err(Status::not_found("Link not found")),
            Err(e) => Err(Status::from(e)),
        }
    }

    async fn list_links(
        &self,
        request: Request<ListLinksRequest>,
    ) -> Result<Response<ListLinksResponse>, Status> {
        let req = request.into_inner();
        let operator_id = Uuid::parse_str(&req.operator_id)
            .map_err(|_| Status::invalid_argument("Invalid operator_id"))?;
        let status_filter = if req.status_filter.is_empty() { None } else { Some(req.status_filter.as_str()) };

        match self.queries.list_links(operator_id, status_filter).await {
            Ok(links) => {
                let proto_links: Vec<GetLinkResponse> = links.into_iter().map(|link| {
                    GetLinkResponse {
                        link_id: link.link_id.to_string(),
                        operator_id: link.operator_id.to_string(),
                        connector_id: link.connector_id,
                        display_name: link.display_name,
                        environment: link.environment.as_str().to_string(),
                        status: link.status.as_str().to_string(),
                        health_status: link.health_status.as_str().to_string(),
                        created_at: Some(platform_proto::common::Timestamp {
                            unix_ms: link.created_at.timestamp_millis(),
                        }),
                        last_tested_at: link.last_tested_at.map(|t| platform_proto::common::Timestamp {
                            unix_ms: t.timestamp_millis(),
                        }),
                        last_healthy_at: link.last_healthy_at.map(|t| platform_proto::common::Timestamp {
                            unix_ms: t.timestamp_millis(),
                        }),
                        credentials_expires_at: link.credentials_expires_at.map(|t| platform_proto::common::Timestamp {
                            unix_ms: t.timestamp_millis(),
                        }),
                    }
                }).collect();

                Ok(Response::new(ListLinksResponse {
                    links: proto_links,
                }))
            }
            Err(e) => Err(Status::from(e)),
        }
    }
}

// ─── Error Conversion ───────────────────────────────────────────────────────

impl From<LinkError> for Status {
    fn from(e: LinkError) -> Self {
        match e {
            LinkError::NotFound(_) => Status::not_found(e.to_string()),
            LinkError::CredentialsInvalid(ref msg) => Status::invalid_argument(msg.clone()),
            LinkError::ConnectionTestFailed(ref msg) => Status::failed_precondition(msg.clone()),
            LinkError::LinkAlreadyDisabled | LinkError::LinkAlreadyEnabled | LinkError::MaxLinksPerConnector => {
                Status::failed_precondition(e.to_string())
            }
            LinkError::CredentialsExpired => Status::failed_precondition("Credentials expired"),
            LinkError::ConnectorNotFound(..) => Status::not_found("Connector not found"),
            LinkError::EncryptionFailed(ref msg) => Status::internal(msg.clone()),
            LinkError::InvalidRequest(ref msg) => Status::invalid_argument(msg.clone()),
        }
    }
}
