use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RouteRequest {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ForwardRequestCommand {
    pub method: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
    pub principal_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateRouteCommand {
    pub path_prefix: String,
    pub target_service: String,
    pub target_url: String,
    pub auth_required: bool,
    pub rate_limit_max_requests: Option<u32>,
    pub rate_limit_window_seconds: Option<u32>,
    pub methods: Option<Vec<String>>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct UpdateRouteCommand {
    pub route_id: Uuid,
    pub path_prefix: Option<String>,
    pub target_service: Option<String>,
    pub target_url: Option<String>,
    pub auth_required: Option<bool>,
    pub rate_limit_max_requests: Option<u32>,
    pub rate_limit_window_seconds: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct DeleteRouteCommand {
    pub route_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct GetRouteHealthCommand {
    pub route_id: Uuid,
}
