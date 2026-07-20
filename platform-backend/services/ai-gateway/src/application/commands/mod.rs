use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct AiGatewayRequest { pub principal_id: Uuid, pub prompt: String, pub model: Option<String> }
