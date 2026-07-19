use serde::{Deserialize, Serialize}; use uuid::Uuid;
#[derive(Debug, Deserialize)]
pub struct SendNotificationRequest { pub notification_type: String, pub recipient: String, pub subject: Option<String>, pub body: String }
#[derive(Debug, Serialize)]
pub struct NotificationResponse { pub notification_id: Uuid, pub status: String, pub notification_type: String, pub recipient: String }
#[derive(Debug, Serialize)]
pub struct ErrorResponse { pub error: String, pub code: String }
