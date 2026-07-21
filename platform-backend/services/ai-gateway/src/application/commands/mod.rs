use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AiGatewayRequest {
    pub principal_id: Uuid,
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GetRequestCommand {
    pub principal_id: Uuid,
    pub request_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ListRequestsCommand {
    pub principal_id: Uuid,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct GetUsageStatsCommand {
    pub principal_id: Uuid,
    pub start: Option<String>,
    pub end: Option<String>,
}
