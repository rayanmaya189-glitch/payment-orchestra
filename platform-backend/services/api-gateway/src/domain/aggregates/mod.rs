#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub path_prefix: String, pub target_service: String, pub target_url: String,
    pub auth_required: bool, pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub method: String, pub path: String, pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>, pub principal_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GatewayResponse {
    pub status: u16, pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
}
