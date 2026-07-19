#![allow(clippy::match_like_matches_macro)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Created,
    Authorizing,
    Authorized,
    Capturing,
    Captured,
    PartiallyCaptured,
    Voided,
    AuthorizationExpired,
    Failed,
    FailedAllRoutes,
    Refunding,
    Refunded,
    PartiallyRefunded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentCommand {
    Authorize,
    Capture,
    Void,
    Refund,
}

impl PaymentStatus {
    pub fn can_transition(&self, command: &PaymentCommand) -> bool {
        match (self, command) {
            (Self::Created, PaymentCommand::Authorize) => true,
            (Self::Authorizing, PaymentCommand::Authorize) => true,
            (Self::Authorized, PaymentCommand::Capture) => true,
            (Self::Authorized, PaymentCommand::Void) => true,
            (Self::PartiallyCaptured, PaymentCommand::Capture) => true,
            (Self::Captured | Self::PartiallyCaptured, PaymentCommand::Refund) => true,
            _ => false,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Authorizing => "authorizing",
            Self::Authorized => "authorized",
            Self::Capturing => "capturing",
            Self::Captured => "captured",
            Self::PartiallyCaptured => "partially_captured",
            Self::Voided => "voided",
            Self::AuthorizationExpired => "authorization_expired",
            Self::Failed => "failed",
            Self::FailedAllRoutes => "failed_all_routes",
            Self::Refunding => "refunding",
            Self::Refunded => "refunded",
            Self::PartiallyRefunded => "partially_refunded",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "created" => Self::Created,
            "authorizing" => Self::Authorizing,
            "authorized" => Self::Authorized,
            "capturing" => Self::Capturing,
            "captured" => Self::Captured,
            "partially_captured" => Self::PartiallyCaptured,
            "voided" => Self::Voided,
            "authorization_expired" => Self::AuthorizationExpired,
            "failed" => Self::Failed,
            "failed_all_routes" => Self::FailedAllRoutes,
            "refunding" => Self::Refunding,
            "refunded" => Self::Refunded,
            "partially_refunded" => Self::PartiallyRefunded,
            _ => Self::Created,
        }
    }
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorized_can_capture() {
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Capture));
    }

    #[test]
    fn test_authorized_can_void() {
        assert!(PaymentStatus::Authorized.can_transition(&PaymentCommand::Void));
    }

    #[test]
    fn test_cannot_capture_failed() {
        assert!(!PaymentStatus::Failed.can_transition(&PaymentCommand::Capture));
    }

    #[test]
    fn test_cannot_void_after_capture() {
        assert!(!PaymentStatus::Captured.can_transition(&PaymentCommand::Void));
    }

    #[test]
    fn test_cannot_refund_voided() {
        assert!(!PaymentStatus::Voided.can_transition(&PaymentCommand::Refund));
    }

    #[test]
    fn test_partially_captured_can_capture_remaining() {
        assert!(PaymentStatus::PartiallyCaptured.can_transition(&PaymentCommand::Capture));
    }

    #[test]
    fn test_created_can_authorize() {
        assert!(PaymentStatus::Created.can_transition(&PaymentCommand::Authorize));
    }

    #[test]
    fn test_cannot_authorize_after_capture() {
        assert!(!PaymentStatus::Captured.can_transition(&PaymentCommand::Authorize));
    }
}
