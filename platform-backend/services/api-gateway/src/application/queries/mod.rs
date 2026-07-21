use crate::domain::aggregates::{GatewayStats, RouteConfig};
use crate::domain::entities::{ForwardResult, RouteHealth};

pub struct RouteListQueryResult {
    pub routes: Vec<RouteConfig>,
}

pub struct ForwardResultQueryResult {
    pub result: Option<ForwardResult>,
}

pub struct RouteHealthQueryResult {
    pub health: Option<RouteHealth>,
}

pub struct GatewayStatsQueryResult {
    pub stats: GatewayStats,
}
