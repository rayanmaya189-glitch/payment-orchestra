//! AI Assistant Service client (BC-12).
//! RAG pipeline, natural-language Q&A, temporal queries.

use crate::client::{ClientError, ServiceConnection};
use platform_proto::ai_assistant::ai_assistant_service_client::AiAssistantServiceClient;
use platform_proto::ai_assistant::{
    AskAssistantRequest, AskAssistantResponse,
    GetSessionHistoryRequest, GetSessionHistoryResponse,
    ClearSessionRequest, ClearSessionResponse,
};

#[derive(Debug, Clone)]
pub struct AiAssistantClient {
    conn: ServiceConnection,
}

impl AiAssistantClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect("ai-assistant-service", addr).await?;
        Ok(Self { conn })
    }

    pub fn connect_lazy(addr: &str) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_lazy("ai-assistant-service", addr)?;
        Ok(Self { conn })
    }

    pub async fn connect_via_etcd(etcd_endpoints: &[String]) -> Result<Self, ClientError> {
        let conn = ServiceConnection::connect_via_etcd("ai-assistant-service", etcd_endpoints).await?;
        Ok(Self { conn })
    }

    async fn client(&self) -> AiAssistantServiceClient<tonic::transport::Channel> {
        AiAssistantServiceClient::new(self.conn.channel().clone())
    }

    pub async fn ask_assistant(&self, req: AskAssistantRequest) -> Result<AskAssistantResponse, ClientError> {
        self.client().await.ask_assistant(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "ai-assistant-service".into(), source: e })
    }

    pub async fn get_session_history(&self, req: GetSessionHistoryRequest) -> Result<GetSessionHistoryResponse, ClientError> {
        self.client().await.get_session_history(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "ai-assistant-service".into(), source: e })
    }

    pub async fn clear_session(&self, req: ClearSessionRequest) -> Result<ClearSessionResponse, ClientError> {
        self.client().await.clear_session(req).await.map(|r| r.into_inner())
            .map_err(|e| ClientError::RpcFailed { service: "ai-assistant-service".into(), source: e })
    }
}
