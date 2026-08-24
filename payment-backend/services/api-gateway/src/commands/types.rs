//! Command types for API Gateway

use crate::domain::*;

pub struct ProcessInboundRequest {
    pub http_method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub source_ip: String,
}

pub struct RegisterRoute {
    pub route: RouteDefinition,
}
