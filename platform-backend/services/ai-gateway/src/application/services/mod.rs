use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use crate::application::commands::*;
use crate::domain::aggregates::AiRequest;
use crate::domain::rules::AiRequestRepository;
use platform_error::PlatformError;

pub struct AiGatewayServiceImpl { repo: Box<dyn AiRequestRepository>, db: DatabaseConnection }
impl AiGatewayServiceImpl { pub fn new(repo: Box<dyn AiRequestRepository>, db: DatabaseConnection) -> Self { Self { repo, db } } }

#[async_trait]
pub trait AiGatewayService: Send + Sync {
    async fn process(&self, cmd: AiGatewayRequest) -> Result<AiRequest, PlatformError>;
}

#[async_trait]
impl AiGatewayService for AiGatewayServiceImpl {
    async fn process(&self, cmd: AiGatewayRequest) -> Result<AiRequest, PlatformError> {
        let mut request = AiRequest::new(cmd.principal_id, cmd.prompt, cmd.model.unwrap_or_else(|| "qwen3".into()));
        request.check_guardrails();
        self.repo.save(&request).await?;
        Ok(request)
    }
}
