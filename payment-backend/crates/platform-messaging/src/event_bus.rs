use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

pub struct EventBus {
    sender: broadcast::Sender<Vec<u8>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), String> {
        let envelope = EventEnvelope {
            subject: subject.to_string(),
            payload,
            event_id: Uuid::now_v7(),
            timestamp: chrono::Utc::now(),
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
        self.sender.send(bytes).map_err(|_| "No subscribers".to_string())?;
        info!("Published to {}", subject);
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.sender.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventEnvelope {
    pub subject: String,
    pub payload: Vec<u8>,
    pub event_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
