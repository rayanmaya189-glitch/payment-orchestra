use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceContext {
    pub source_type: SourceType,
    pub source_id: Option<Uuid>,
    pub source_metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    MerchantApi,
    Invoice,
    Subscription,
    PaymentLink,
    AiAssistant,
    System,
}
