use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{parse_uuid, profile_to_view, card_scheme_from_str};
use super::GatewayProfileGrpcService;
use crate::commands::{self, CommandHandler};
use crate::queries::QueryHandler;
use crate::domain::{
    FeeStructure, MonitoringThresholds, RateLimitConfig, TransactionLimits,
};

use platform_proto::gateway_profile::gateway_profile_service_server::GatewayProfileService;
use platform_proto::gateway_profile::*;
use platform_proto::common::Timestamp;

#[allow(clippy::too_many_lines)]
#[tonic::async_trait]
impl<C, Q> GatewayProfileService for GatewayProfileGrpcService<C, Q>
where
    C: CommandHandler + Send + Sync + 'static,
    Q: QueryHandler + Send + Sync + 'static,
{
    async fn create_gateway_profile(
        &self,
        request: Request<CreateGatewayProfileRequest>,
    ) -> Result<Response<CreateGatewayProfileResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;
        let link_id = if req.merchant_acquirer_link_id.is_empty() {
            Uuid::now_v7()
        } else {
            parse_uuid(&req.merchant_acquirer_link_id, "merchant_acquirer_link_id")?
        };

        let card_schemes = req.enabled_card_schemes.iter()
            .filter_map(|s| card_scheme_from_str(s))
            .collect();

        let cmd = commands::CreateGatewayProfile {
            operator_id,
            connector_id: req.connector_id,
            merchant_acquirer_link_id: link_id,
            limits: TransactionLimits {
                min_amount_minor: req.min_amount_minor,
                max_amount_minor: req.max_amount_minor,
                daily_volume_limit_minor: req.daily_volume_limit_minor,
                monthly_volume_limit_minor: req.daily_volume_limit_minor * 30,
                max_refund_amount_minor: req.max_amount_minor,
            },
            fees: FeeStructure {
                fixed_fee_minor: req.fixed_fee_minor,
                percentage_fee_bps: req.percentage_fee_bps,
                cross_border_fee_bps: 0,
                currency_conversion_fee_bps: 0,
                max_fee_cap: None,
                min_fee_floor: None,
                tiered_pricing: None,
            },
            routing_priority: req.routing_priority,
            enabled_card_schemes: card_schemes,
            enabled_currencies: req.enabled_currencies,
            enabled_countries: req.enabled_countries,
            rate_limits: RateLimitConfig {
                per_second: req.rate_limit_per_second,
                per_day: req.rate_limit_per_day,
                burst_size: req.burst_size,
            },
            monitoring: MonitoringThresholds {
                success_rate_alert: 0.95,
                success_rate_critical: 0.90,
                latency_p99_alert_ms: 3000,
                latency_p99_critical_ms: 5000,
                auto_disable_on_low_success: false,
            },
        };

        match self.commands.create_gateway_profile(cmd).await {
            Ok(result) => {
                Ok(Response::new(CreateGatewayProfileResponse {
                    profile_id: result.profile.profile_id.to_string(),
                    operator_id: result.profile.operator_id.to_string(),
                    connector_id: result.profile.connector_id,
                    status: result.profile.status.as_str().to_string(),
                    created_at: Some(Timestamp {
                        unix_ms: result.profile.created_at.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn get_gateway_profile(
        &self,
        request: Request<GetGatewayProfileRequest>,
    ) -> Result<Response<GatewayProfileView>, Status> {
        let req = request.into_inner();
        let profile_id = parse_uuid(&req.profile_id, "profile_id")?;

        match self.queries.get_gateway_profile(profile_id).await {
            Ok(Some(profile)) => Ok(Response::new(profile_to_view(profile))),
            Ok(None) => Err(Status::not_found("Gateway profile not found")),
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn list_gateway_profiles(
        &self,
        request: Request<ListGatewayProfilesRequest>,
    ) -> Result<Response<ListGatewayProfilesResponse>, Status> {
        let req = request.into_inner();
        let operator_id = parse_uuid(&req.operator_id, "operator_id")?;

        match self.queries.list_gateway_profiles(operator_id).await {
            Ok(profiles) => {
                let proto_profiles: Vec<GatewayProfileView> = profiles
                    .into_iter()
                    .map(profile_to_view)
                    .collect();

                Ok(Response::new(ListGatewayProfilesResponse {
                    profiles: proto_profiles,
                }))
            }
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn update_gateway_profile(
        &self,
        request: Request<UpdateGatewayProfileRequest>,
    ) -> Result<Response<UpdateGatewayProfileResponse>, Status> {
        let req = request.into_inner();
        let profile_id = parse_uuid(&req.profile_id, "profile_id")?;

        let card_schemes: Option<Vec<crate::domain::CardScheme>> = {
            let schemes: Vec<crate::domain::CardScheme> = req.enabled_card_schemes.iter()
                .filter_map(|s| card_scheme_from_str(s))
                .collect();
            if schemes.is_empty() { None } else { Some(schemes) }
        };
        let currencies = if req.enabled_currencies.is_empty() {
            None
        } else {
            Some(req.enabled_currencies)
        };
        let countries = if req.enabled_countries.is_empty() {
            None
        } else {
            Some(req.enabled_countries)
        };

        let cmd = commands::UpdateGatewayProfile {
            profile_id,
            limits: Some(TransactionLimits {
                min_amount_minor: req.min_amount_minor,
                max_amount_minor: req.max_amount_minor,
                daily_volume_limit_minor: req.daily_volume_limit_minor,
                monthly_volume_limit_minor: req.daily_volume_limit_minor * 30,
                max_refund_amount_minor: req.max_amount_minor,
            }),
            fees: Some(FeeStructure {
                fixed_fee_minor: req.fixed_fee_minor,
                percentage_fee_bps: req.percentage_fee_bps,
                cross_border_fee_bps: 0,
                currency_conversion_fee_bps: 0,
                max_fee_cap: None,
                min_fee_floor: None,
                tiered_pricing: None,
            }),
            rate_limits: None,
            monitoring: None,
            status: None,
            routing_priority: Some(req.routing_priority),
            enabled_card_schemes: card_schemes,
            enabled_currencies: currencies,
            enabled_countries: countries,
        };

        match self.commands.update_gateway_profile(cmd).await {
            Ok(result) => {
                Ok(Response::new(UpdateGatewayProfileResponse {
                    profile_id: result.profile.profile_id.to_string(),
                    status: result.profile.status.as_str().to_string(),
                    updated_at: Some(Timestamp {
                        unix_ms: result.profile.updated_at.timestamp_millis(),
                    }),
                }))
            }
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn list_connectors(
        &self,
        _request: Request<ListConnectorsRequest>,
    ) -> Result<Response<ListConnectorsResponse>, Status> {
        match self.queries.list_connectors().await {
            Ok(connectors) => {
                let proto_connectors: Vec<ConnectorInfoView> = connectors
                    .into_iter()
                    .map(|c| ConnectorInfoView {
                        connector_id: c.connector_id.clone(),
                        display_name: super::connector_display_name(&c.connector_id),
                        supported_card_schemes: vec![],
                        supported_currencies: vec![],
                        supports_partial_capture: false,
                        supports_partial_refund: false,
                        supports_webhook_settlement: false,
                        settlement_cycle: c.settlement_cycle,
                        cross_border_fee_bps: 0,
                        test_card_count: c.test_card_count as u32,
                    })
                    .collect();

                Ok(Response::new(ListConnectorsResponse {
                    connectors: proto_connectors,
                }))
            }
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn get_connector_schema(
        &self,
        request: Request<GetConnectorSchemaRequest>,
    ) -> Result<Response<ConnectorSchemaView>, Status> {
        let req = request.into_inner();

        match self.queries.get_connector_schema(&req.connector_id).await {
            Ok(Some(schema)) => {
                let fields: Vec<OnboardingFieldView> = schema.fields.iter().map(|f| {
                    let (field_type, options) = match &f.field_type {
                        crate::domain::FieldType::String => ("string".into(), vec![]),
                        crate::domain::FieldType::Password => ("password".into(), vec![]),
                        crate::domain::FieldType::Url => ("url".into(), vec![]),
                        crate::domain::FieldType::Integer => ("integer".into(), vec![]),
                        crate::domain::FieldType::Select { options: opts } => (
                            "select".into(),
                            opts.iter().map(|o| SelectOptionView {
                                value: o.value.clone(),
                                label: o.label.clone(),
                            }).collect(),
                        ),
                    };

                    OnboardingFieldView {
                        name: f.name.clone(),
                        field_type,
                        required: f.required,
                        label: f.label.clone(),
                        validation_regex: f.validation_regex.clone().unwrap_or_default(),
                        help_text: f.help_text.clone().unwrap_or_default(),
                        options,
                    }
                }).collect();

                Ok(Response::new(ConnectorSchemaView {
                    connector_id: schema.connector_id,
                    fields,
                }))
            }
            Ok(None) => Err(Status::not_found(format!(
                "Connector '{}' not found",
                req.connector_id
            ))),
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn validate_credentials(
        &self,
        request: Request<ValidateCredentialsRequest>,
    ) -> Result<Response<ValidateCredentialsResponse>, Status> {
        let req = request.into_inner();

        let config = crate::domain::ConnectorConfig {
            api_key: if req.api_key.is_empty() { None } else { Some(req.api_key) },
            secret_key: if req.secret_key.is_empty() { None } else { Some(req.secret_key) },
            merchant_id: if req.merchant_id.is_empty() { None } else { Some(req.merchant_id) },
            store_id: if req.store_id.is_empty() { None } else { Some(req.store_id) },
            environment: if req.environment.is_empty() { "sandbox".into() } else { req.environment },
            additional_fields: req.additional_fields,
        };

        let cmd = commands::ValidateCredentials {
            connector_id: req.connector_id,
            config,
        };

        match self.commands.validate_credentials(cmd).await {
            Ok(result) => {
                let perms = if result.valid {
                    vec!["authorize".into(), "capture".into(), "refund".into()]
                } else {
                    vec![]
                };
                Ok(Response::new(ValidateCredentialsResponse {
                    valid: result.valid,
                    merchant_name: result.merchant_name.clone().unwrap_or_default(),
                    permissions: perms,
                    error_message: String::new(),
                }))
            }
            Err(e) => Err(Status::internal(e)),
        }
    }

    async fn test_connection(
        &self,
        request: Request<TestConnectionRequest>,
    ) -> Result<Response<TestConnectionResponse>, Status> {
        let req = request.into_inner();

        let config = crate::domain::ConnectorConfig {
            api_key: if req.api_key.is_empty() { None } else { Some(req.api_key) },
            secret_key: if req.secret_key.is_empty() { None } else { Some(req.secret_key) },
            merchant_id: if req.merchant_id.is_empty() { None } else { Some(req.merchant_id) },
            store_id: if req.store_id.is_empty() { None } else { Some(req.store_id) },
            environment: if req.environment.is_empty() { "sandbox".into() } else { req.environment },
            additional_fields: req.additional_fields,
        };

        match self.registry.get(&req.connector_id) {
            Ok(connector) => {
                let result = connector.test_connection(&config).await.map_err(|e| {
                    Status::internal(format!("Connection test failed: {}", e))
                })?;
                Ok(Response::new(TestConnectionResponse {
                    success: result.success,
                    merchant_name: result.merchant_name.unwrap_or_default(),
                    latency_ms: result.latency_ms,
                    error_message: result.error_message.unwrap_or_default(),
                }))
            }
            Err(_) => Err(Status::not_found(format!(
                "Connector '{}' not found",
                req.connector_id
            ))),
        }
    }
}
