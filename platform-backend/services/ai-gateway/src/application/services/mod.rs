use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use crate::application::commands::*;
use crate::domain::aggregates::AiRequest;
use crate::domain::rules::AiRequestRepository;
use platform_error::PlatformError;
use platform_middleware::{evaluate_policy, AbacContext};

pub struct AiGatewayServiceImpl { repo: Box<dyn AiRequestRepository>, db: DatabaseConnection }
impl AiGatewayServiceImpl { pub fn new(repo: Box<dyn AiRequestRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait AiGatewayService: Send + Sync {
    async fn process(&self, cmd: AiGatewayRequest) -> Result<AiRequest, PlatformError>;
}

#[async_trait]
impl AiGatewayService for AiGatewayServiceImpl {
    async fn process(&self, cmd: AiGatewayRequest) -> Result<AiRequest, PlatformError> {
        // ABAC: AI access requires "read" permission on "ai_assistant"
        // Any authenticated principal can use AI, but it's auditable
        let ctx = AbacContext {
            principal_id: cmd.principal_id,
            role: "api_client".to_string(), // API key clients default to api_client
            action: "read".to_string(),
            resource: "ai_assistant".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)?;

        let mut request = AiRequest::new(cmd.principal_id, cmd.prompt, cmd.model.unwrap_or_else(|| "qwen3".into()));
        request.check_guardrails();
        self.repo.save(&request).await?;
        Ok(request)
    }
}
