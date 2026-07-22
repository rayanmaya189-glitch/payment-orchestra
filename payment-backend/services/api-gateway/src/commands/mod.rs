//! API Gateway commands

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;

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

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn process_request(&self, cmd: ProcessInboundRequest) -> Result<ProcessedRequest, GatewayError>;
    async fn register_route(&self, cmd: RegisterRoute) -> Result<(), GatewayError>;
}

pub struct GatewayCommandHandler<R: GatewayRepository> {
    repo: R,
}

impl<R: GatewayRepository> GatewayCommandHandler<R> {
    pub fn new(repo: R) -> Self { Self { repo } }
}

#[async_trait]
impl<R: GatewayRepository + Send + Sync> CommandHandler for GatewayCommandHandler<R> {
    async fn process_request(&self, cmd: ProcessInboundRequest) -> Result<ProcessedRequest, GatewayError> {
        let request_id = Uuid::now_v7();
        let now = Utc::now();

        // Step 1: Validate HTTP method
        if !ALLOWED_METHODS.contains(&cmd.http_method.as_str()) {
            return Ok(ProcessedRequest {
                request_id, route: None, authenticated: false, actor_id: None,
                actor_type: None, rate_limited: false, allowed: false,
                http_status: 405, response_body: format!("Method {} not allowed", cmd.http_method).into_bytes(),
                processed_at: now,
            });
        }

        // Step 2: Check body size
        if cmd.body.len() > MAX_BODY_SIZE {
            return Ok(ProcessedRequest {
                request_id, route: None, authenticated: false, actor_id: None,
                actor_type: None, rate_limited: false, allowed: false,
                http_status: 413,
                response_body: format!("Body too large: {} bytes (max: {})", cmd.body.len(), MAX_BODY_SIZE).into_bytes(),
                processed_at: now,
            });
        }

        // Step 3: Route matching
        let route = self.repo.get_route(&cmd.http_method, &cmd.path).await?
            .ok_or_else(|| GatewayError::RouteNotFound(cmd.http_method.clone(), cmd.path.clone()))?;

        // Step 4: Authentication
        let auth_result = if route.auth_required {
            let api_key = extract_api_key(&cmd.headers);
            match api_key {
                Some(key) => self.repo.validate_api_key(key).await?,
                None => AuthResult {
                    authenticated: false, actor_id: None, actor_type: None,
                    scopes: vec![], error: Some("Missing API key or auth token".into()),
                },
            }
        } else {
            AuthResult {
                authenticated: true, actor_id: Some(Uuid::now_v7()), actor_type: Some(ActorType::System),
                scopes: vec!["public".into()], error: None,
            }
        };

        if route.auth_required && !auth_result.authenticated {
            return Ok(ProcessedRequest {
                request_id, route: Some(route.clone()), authenticated: false,
                actor_id: None, actor_type: None, rate_limited: false, allowed: false,
                http_status: 401,
                response_body: auth_result.error.unwrap_or_else(|| "Authentication failed".into()).into_bytes(),
                processed_at: now,
            });
        }

        // Step 5: Rate limiting
        let rate_limited = if let Some(ref rl_config) = route.rate_limit_config {
            let rate_key = format!("{}:{}", rl_config.per.as_str(), auth_result.actor_id.map(|id| id.to_string()).unwrap_or_default());
            let allowed = self.repo.check_rate_limit(&rate_key, rl_config).await?;
            if !allowed {
                return Ok(ProcessedRequest {
                    request_id, route: Some(route.clone()), authenticated: true,
                    actor_id: auth_result.actor_id, actor_type: auth_result.actor_type.map(|t| format!("{:?}", t)),
                    rate_limited: true, allowed: false, http_status: 429,
                    response_body: b"Rate limit exceeded".to_vec(),
                    processed_at: now,
                });
            }
            false
        } else {
            false
        };

        // Step 6: Build response (Phase 1: simulates forwarding to backend)
        let result = ProcessedRequest {
            request_id: request_id.clone(),
            route: Some(route.clone()),
            authenticated: true,
            actor_id: auth_result.actor_id,
            actor_type: auth_result.actor_type.map(|t| format!("{:?}", t)),
            rate_limited,
            allowed: true,
            http_status: 200,
            response_body: format!("Forwarded to {}::{}", route.grpc_service, route.grpc_method).into_bytes(),
            processed_at: Utc::now(),
        };

        // Step 7: Log request
        self.repo.log_request(&result).await?;

        Ok(result)
    }

    async fn register_route(&self, _cmd: RegisterRoute) -> Result<(), GatewayError> {
        // Phase 1: routes are defined statically in default_routes().
        // Dynamic route registration via API will be implemented in a future phase.
        Ok(())
    }
}

fn extract_api_key(headers: &[(String, String)]) -> Option<&str> {
    for (name, value) in headers {
        let lower = name.to_lowercase();
        if lower == "authorization" {
            if let Some(key) = value.strip_prefix("Bearer ") {
                return Some(key);
            }
            return Some(value); // Raw API key without Bearer prefix
        }
        if lower == "x-api-key" {
            return Some(value);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Rate limit scope display
// ---------------------------------------------------------------------------

impl RateLimitScope {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ip => "ip",
            Self::ApiKey => "api_key",
            Self::Operator => "operator",
        }
    }
}
