use serde::{Deserialize, Serialize};

use super::error::OrchestrationError;

/// Payment status state machine.
///
/// ```text
///                                   ┌──────────────┐
///                                   │   Created    │
///                                   └──────┬───────┘
///                                          │ Authorize
///                                          ▼
///                                  ┌───────────────┐
///                                  │  Authorizing  │
///                                  └───────┬───────┘
///                            ┌─────────────┼─────────────┐
///                            │             │             │
///                      success│      3ds   │      fail  │
///                            ▼             ▼             ▼
///                    ┌──────────────┐ ┌──────────┐ ┌────────────┐
///                    │  Authorized  │ │Requires3DS│ │   Failed   │
///                    └──────┬───────┘ └────┬─────┘ └──────┬─────┘
///                ┌──────────┼──────────┐   │              │
///                │          │          │   │ retry        │ all routes
///          Capture│     Void │   Expire │   │ exhausted    │ failed
///                ▼          ▼          ▼   │              ▼
///     ┌──────────────┐ ┌────────┐ ┌──────┐ │    ┌─────────────────┐
///     │  Capturing   │ │ Voided │ │ Auth │ │    │ FailedAllRoutes │
///     └──────┬───────┘ └────────┘ │Expire│ │    └─────────────────┘
///            │                    └──────┘ │
///     ┌──────┴──────┐                     │
///     │             │                     │
/// Capture     Partial                    │
/// complete    Capture                     │
///     │             │                     │
///     ▼             ▼                     │
/// ┌────────┐ ┌────────────────┐          │
/// │Captured│ │PartiallyCaptured│          │
/// └───┬────┘ └───────┬────────┘          │
///     │               │                   │
///     │Refund    Capture/Refund           │
///     ▼               ▼                   │
/// ┌────────┐ ┌────────────────┐          │
/// │Refunded│ │PartiallyRefunded│          │
/// └────────┘ └────────────────┘          │
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PaymentStatus {
    /// Payment intent created, not yet sent to any gateway.
    Created,
    /// Authorization request sent to a gateway, awaiting response.
    Authorizing,
    /// Gateway approved the authorization.
    Authorized,
    /// Capture request sent to the gateway.
    Capturing,
    /// Full amount captured successfully.
    Captured,
    /// Partial amount captured (merchant can capture more later).
    PartiallyCaptured,
    /// Authorization voided (no funds captured).
    Voided,
    /// Authorization expired without capture (typically 7 days).
    AuthorizationExpired,
    /// Single gateway declined, will attempt next route.
    Failed,
    /// All routes exhausted, payment permanently failed.
    FailedAllRoutes,
    /// Refund request sent to the gateway.
    Refunding,
    /// Full refund processed.
    Refunded,
    /// Partial refund processed.
    PartiallyRefunded,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PaymentStatus {
    /// Returns true if the given command is allowed from this state.
    /// Uses zero-allocation match arms instead of building a Vec.
    fn is_transition_allowed(&self, command: &str) -> bool {
        matches!(
            (self, command),
            (Self::Created, "Authorize")
                | (Self::Authorizing, "Authorize")
                | (Self::Authorized, "Capture" | "Void")
                | (Self::Captured, "Refund")
                | (Self::PartiallyCaptured, "Capture" | "Refund")
                | (Self::PartiallyRefunded, "Refund")
        )
    }

    /// Validate a state transition for a given command.
    /// Returns Ok(()) if the transition is valid, Err with the appropriate error code otherwise.
    pub fn can_execute_command(&self, command: &str) -> Result<(), OrchestrationError> {
        if self.is_transition_allowed(command) {
            return Ok(());
        }

        // Specific error codes for common invalid transitions
        let error_code = match self {
            Self::Failed | Self::FailedAllRoutes => "PAYMENT_INTENT_FAILED",
            Self::Voided => "PAYMENT_INTENT_VOIDED",
            Self::AuthorizationExpired => "AUTHORIZATION_EXPIRED",
            Self::Captured => "PAYMENT_INTENT_ALREADY_CAPTURED",
            Self::Refunded => "PAYMENT_INTENT_FULLY_REFUNDED",
            Self::Authorizing => "PAYMENT_INTENT_AUTHORIZING",
            Self::Capturing => "PAYMENT_INTENT_CAPTURING",
            Self::Created if matches!(command, "Capture" | "Void" | "Refund") => {
                "PAYMENT_INTENT_NOT_AUTHORIZED"
            }
            _ => return Err(OrchestrationError::InvalidStateTransition(format!(
                "Cannot {} in state {:?}",
                command, self
            ))),
        };

        Err(OrchestrationError::InvalidStateTransition(
            error_code.to_string(),
        ))
    }

    /// Whether this is a terminal state (no more transitions possible).
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

    /// Whether this is a failure state.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed | Self::FailedAllRoutes)
    }

    /// Whether the payment can still be retried on another gateway.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Whether the payment has been authorized (partial or full).
    pub fn is_authorized(&self) -> bool {
        matches!(
            self,
            Self::Authorized | Self::PartiallyCaptured | Self::Captured
        )
    }

    /// Get the next expected state after a command (before the command result is known).
    /// For commands with ambiguous outcomes (e.g., Capture), returns the intermediate
    /// state while the command is in progress.
    pub fn next_state_after(&self, command: &str) -> Option<PaymentStatus> {
        match (self, command) {
            (Self::Created, "Authorize") | (Self::Authorizing, "Authorize") => {
                Some(PaymentStatus::Authorizing)
            }
            (Self::Authorized, "Capture") | (Self::PartiallyCaptured, "Capture") => {
                Some(PaymentStatus::Capturing)
            }
            (Self::Authorized, "Void") => Some(PaymentStatus::Voided),
            _ => None,
        }
    }
}
