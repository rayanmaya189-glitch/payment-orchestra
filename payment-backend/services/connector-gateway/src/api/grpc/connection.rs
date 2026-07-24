use tonic::{Request, Response, Status};

use super::GatewayProfileGrpcService;
use crate::commands::CommandHandler;
use crate::queries::QueryHandler;

use platform_proto::gateway_profile::gateway_profile_service_server::GatewayProfileService;
use platform_proto::gateway_profile::*;

#[tonic::async_trait]
impl<C, Q> GatewayProfileService for GatewayProfileGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn create_gateway_profile(
        &self,
        _request: Request<CreateGatewayProfileRequest>,
    ) -> Result<Response<CreateGatewayProfileResponse>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn get_gateway_profile(
        &self,
        _request: Request<GetGatewayProfileRequest>,
    ) -> Result<Response<GatewayProfileView>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn list_gateway_profiles(
        &self,
        _request: Request<ListGatewayProfilesRequest>,
    ) -> Result<Response<ListGatewayProfilesResponse>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn update_gateway_profile(
        &self,
        _request: Request<UpdateGatewayProfileRequest>,
    ) -> Result<Response<UpdateGatewayProfileResponse>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn list_connectors(
        &self,
        _request: Request<ListConnectorsRequest>,
    ) -> Result<Response<ListConnectorsResponse>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn get_connector_schema(
        &self,
        _request: Request<GetConnectorSchemaRequest>,
    ) -> Result<Response<ConnectorSchemaView>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn validate_credentials(
        &self,
        _request: Request<ValidateCredentialsRequest>,
    ) -> Result<Response<ValidateCredentialsResponse>, Status> {
        unreachable!("implemented in profile module")
    }

    async fn test_connection(
        &self,
        _request: Request<TestConnectionRequest>,
    ) -> Result<Response<TestConnectionResponse>, Status> {
        unreachable!("implemented in profile module")
    }
}
