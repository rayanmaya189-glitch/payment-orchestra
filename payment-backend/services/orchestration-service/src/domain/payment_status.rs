use serde::{Deserialize, Serialize};

use super::error::OrchestrationError;

/// Payment status state machine.
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

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PaymentStatus {
    /// Validate a state transition for a given command.
    /// Returns Ok(()) if the transition is valid, Err with the appropriate error code otherwise.
    pub fn can_execute_command(&self, command: &str) -> Result<(), OrchestrationError> {
        match (self, command) {
            // Valid transitions
            (Self::Created, "Authorize") => Ok(()),
            (Self::Authorizing, "Authorize") => Ok(()),
            (Self::Authorized, "Capture") => Ok(()),
            (Self::Authorized, "Void") => Ok(()),
            (Self::PartiallyCaptured, "Capture") => Ok(()),
            (Self::Captured, "Refund") => Ok(()),
            (Self::PartiallyCaptured, "Refund") => Ok(()),

            // Invalid transitions with specific error codes
            (Self::Failed, cmd) | (Self::FailedAllRoutes, cmd) if cmd == "Capture" || cmd == "Void" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_FAILED".into())),
            (Self::Voided, cmd) if cmd == "Capture" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_VOIDED".into())),
            (Self::AuthorizationExpired, cmd) if cmd == "Capture" || cmd == "Void" =>
                Err(OrchestrationError::InvalidStateTransition("AUTHORIZATION_EXPIRED".into())),
            (Self::Captured, _) =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_ALREADY_CAPTURED".into())),
            (Self::Refunded, cmd) if cmd == "Refund" || cmd == "Capture" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_FULLY_REFUNDED".into())),
            (Self::Authorizing, cmd) if cmd == "Capture" || cmd == "Void" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_AUTHORIZING".into())),
            (Self::Capturing, cmd) if cmd == "Void" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_CAPTURING".into())),
            (Self::Created, cmd) if cmd == "Capture" || cmd == "Void" || cmd == "Refund" =>
                Err(OrchestrationError::InvalidStateTransition("PAYMENT_INTENT_NOT_AUTHORIZED".into())),

            // Default: transition not defined
            _ => Err(OrchestrationError::InvalidStateTransition(format!("Cannot {} in state {:?}", command, self))),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Captured | Self::Voided | Self::AuthorizationExpired
            | Self::Failed | Self::FailedAllRoutes | Self::Refunded)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed | Self::FailedAllRoutes)
    }
}
