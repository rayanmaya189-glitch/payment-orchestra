use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PaymentCommand {
    Authorize,
    Capture,
    Void,
    Refund,
}

impl fmt::Display for PaymentCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Full lifecycle state machine for PaymentIntent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
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

impl PaymentStatus {
    pub fn can_transition(&self, command: &PaymentCommand) -> bool {
        matches!(
            (self, command),
            (Self::Created, PaymentCommand::Authorize)
                | (Self::Authorizing, PaymentCommand::Authorize)
                | (Self::Authorized, PaymentCommand::Capture)
                | (Self::Authorized, PaymentCommand::Void)
                | (Self::PartiallyCaptured, PaymentCommand::Capture)
                | (Self::Captured | Self::PartiallyCaptured, PaymentCommand::Refund)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Captured
                | Self::Voided
                | Self::AuthorizationExpired
                | Self::Failed
                | Self::FailedAllRoutes
                | Self::Refunded
        )
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed | Self::FailedAllRoutes)
    }

    pub fn can_retry(&self) -> bool {
        matches!(self, Self::Authorizing)
    }
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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
    fn test_terminal_states() {
        assert!(PaymentStatus::Captured.is_terminal());
        assert!(PaymentStatus::FailedAllRoutes.is_terminal());
        assert!(!PaymentStatus::Authorized.is_terminal());
    }

    #[test]
    fn test_failed_states() {
        assert!(PaymentStatus::Failed.is_failed());
        assert!(!PaymentStatus::Authorized.is_failed());
    }
}
