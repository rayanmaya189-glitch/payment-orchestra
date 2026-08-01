//! gRPC service implementation for connector-gateway.
//!
//! Implements the GatewayProfileService (defined in gateway_profile.proto)
//! by translating between protobuf types and the domain CommandHandler/QueryHandler.
//!
//! Endpoints:
//! - Gateway Profile CRUD: create, get, update, list
//! - Connector discovery: list, schema
//! - Credential operations: validate, test connection

use tonic::Status;
use uuid::Uuid;

use crate::domain::{
    CardScheme, GatewayProfile,
};

use platform_proto::gateway_profile::*;
use platform_proto::common::Timestamp;

pub mod profile;

pub struct GatewayProfileGrpcService<C, Q> {
    commands: C,
    queries: Q,
    registry: crate::domain::ConnectorRegistry,
}

impl<C, Q> GatewayProfileGrpcService<C, Q> {
    pub fn new(commands: C, queries: Q, registry: crate::domain::ConnectorRegistry) -> Self {
        Self { commands, queries, registry }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub(crate) fn parse_uuid(s: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid {}", field)))
}

pub(crate) fn profile_to_view(profile: GatewayProfile) -> GatewayProfileView {
    let card_schemes: Vec<String> = profile.enabled_card_schemes.iter()
        .map(|s| s.to_string())
        .collect();

    GatewayProfileView {
        profile_id: profile.profile_id.to_string(),
        operator_id: profile.operator_id.to_string(),
        connector_id: profile.connector_id,
        status: profile.status.as_str().to_string(),
        min_amount_minor: profile.limits.min_amount_minor,
        max_amount_minor: profile.limits.max_amount_minor,
        daily_volume_limit_minor: profile.limits.daily_volume_limit_minor,
        fixed_fee_minor: profile.fees.fixed_fee_minor,
        percentage_fee_bps: profile.fees.percentage_fee_bps,
        enabled_card_schemes: card_schemes,
        enabled_currencies: profile.enabled_currencies,
        enabled_countries: profile.enabled_countries,
        routing_priority: profile.routing_priority,
        created_at: Some(Timestamp {
            unix_ms: profile.created_at.timestamp_millis(),
        }),
        updated_at: Some(Timestamp {
            unix_ms: profile.updated_at.timestamp_millis(),
        }),
    }
}

pub(crate) fn card_scheme_from_str(s: &str) -> Option<CardScheme> {
    match s.to_lowercase().as_str() {
        "visa" => Some(CardScheme::Visa),
        "mastercard" => Some(CardScheme::Mastercard),
        "amex" | "american_express" => Some(CardScheme::Amex),
        other => Some(CardScheme::Other(other.to_string())),
    }
}

pub(crate) fn connector_display_name(connector_id: &str) -> String {
    match connector_id {
        "stripe" => "Stripe".into(),
        "checkout_com" => "Checkout.com".into(),
        "network_international" => "Network International".into(),
        "telr" => "Telr".into(),
        "tap_payments" => "Tap Payments".into(),
        "paytabs" => "PayTabs".into(),
        "mamo" => "Mamo".into(),
        "amazon_ps" => "Amazon Payment Services".into(),
        "aani" => "Aani".into(),
        other => other.to_string(),
    }
}
