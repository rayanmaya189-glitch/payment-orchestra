use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SendNotificationCommand {
    pub operator_id: Uuid,
    pub notification_type: String,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub template_id: Option<String>,
    pub template_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct RetryNotificationCommand {
    pub notification_id: Uuid,
}
