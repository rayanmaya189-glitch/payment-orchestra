//! Notification delivery lifecycle status — Queued → Sent | Failed | DeadLetter.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    /// Notification queued, awaiting delivery.
    Queued,
    /// Successfully delivered.
    Sent,
    /// Delivery failed, retry pending.
    Failed,
    /// Max retries exceeded, moved to dead letter.
    DeadLetter,
}

impl DeliveryStatus {
    pub fn can_retry(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Sent => write!(f, "sent"),
            Self::Failed => write!(f, "failed"),
            Self::DeadLetter => write!(f, "dead_letter"),
        }
    }
}

impl std::str::FromStr for DeliveryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(DeliveryStatus::Queued),
            "sent" => Ok(DeliveryStatus::Sent),
            "failed" => Ok(DeliveryStatus::Failed),
            "dead_letter" => Ok(DeliveryStatus::DeadLetter),
            _ => Err(format!("Invalid delivery status: {}", s)),
        }
    }
}
