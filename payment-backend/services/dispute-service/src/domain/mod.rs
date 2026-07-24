#![allow(clippy::too_many_lines)]
//! Dispute Management domain model — BC-10
//!
//! Event-sourced ChargebackCase aggregate with representment lifecycle.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ChargebackStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargebackStatus {
    /// Chargeback received from acquirer.
    Received,
    /// Case is under manual review.
    UnderReview,
    /// Representment evidence has been submitted to acquirer.
    RepresentmentSubmitted,
    /// Chargeback was won (funds returned to merchant).
    Won,
    /// Chargeback was lost (stand).
    Lost,
    /// Merchant accepted the chargeback.
    Accepted,
    /// Case escalated to scheme arbitration.
    Escalated,
}

impl ChargebackStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use ChargebackStatus::*;
        match (self, target) {
            (Received, UnderReview) => true,
            (Received, RepresentmentSubmitted) => true,
            (Received, Accepted) => true,
            (UnderReview, RepresentmentSubmitted) => true,
            (UnderReview, Accepted) => true,
            (RepresentmentSubmitted, Won) => true,
            (RepresentmentSubmitted, Lost) => true,
            (RepresentmentSubmitted, Escalated) => true,
            // Terminal states: no transitions out
            (Won, _) | (Lost, _) | (Accepted, _) | (Escalated, _) => false,
            _ => false,
        }
    }

    /// Returns true if this is a terminal (resolved) status.
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Won | Self::Lost | Self::Accepted)
    }
}

impl std::str::FromStr for ChargebackStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "received" => Ok(ChargebackStatus::Received),
            "under_review" => Ok(ChargebackStatus::UnderReview),
            "representment_submitted" => Ok(ChargebackStatus::RepresentmentSubmitted),
            "won" => Ok(ChargebackStatus::Won),
            "lost" => Ok(ChargebackStatus::Lost),
            "accepted" => Ok(ChargebackStatus::Accepted),
            "escalated" => Ok(ChargebackStatus::Escalated),
            _ => Err(format!("Invalid chargeback status: {}", s)),
        }
    }
}

impl std::fmt::Display for ChargebackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Received => write!(f, "received"),
            Self::UnderReview => write!(f, "under_review"),
            Self::RepresentmentSubmitted => write!(f, "representment_submitted"),
            Self::Won => write!(f, "won"),
            Self::Lost => write!(f, "lost"),
            Self::Accepted => write!(f, "accepted"),
            Self::Escalated => write!(f, "escalated"),
        }
    }
}

// ---------------------------------------------------------------------------
// ChargebackOutcome & ReasonCode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargebackOutcome {
    Won,
    Lost,
    Accepted,
    Escalated,
}

/// Standard chargeback reason codes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChargebackReasonCode {
    Fraud,
    Duplicate,
    ProductNotReceived,
    ProductNotAsDescribed,
    CreditNotProcessed,
    InvalidAuthorization,
    ProcessingError,
    Other(String),
}

impl std::fmt::Display for ChargebackReasonCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fraud => write!(f, "fraud"),
            Self::Duplicate => write!(f, "duplicate"),
            Self::ProductNotReceived => write!(f, "product_not_received"),
            Self::ProductNotAsDescribed => write!(f, "product_not_as_described"),
            Self::CreditNotProcessed => write!(f, "credit_not_processed"),
            Self::InvalidAuthorization => write!(f, "invalid_authorization"),
            Self::ProcessingError => write!(f, "processing_error"),
            Self::Other(s) => write!(f, "other:{}", s),
        }
    }
}

// ---------------------------------------------------------------------------
// RepresentmentEvidence
// ---------------------------------------------------------------------------

/// Evidence package for representment submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepresentmentEvidence {
    pub transaction_receipt: Option<Uuid>,
    pub delivery_confirmation: Option<Uuid>,
    pub customer_communication: Option<Uuid>,
    pub cardholder_agreement: Option<Uuid>,
    pub refund_policy: Option<Uuid>,
    pub description: String,
    pub supporting_documents: Vec<Uuid>,
}

impl RepresentmentEvidence {
    /// Validate that the evidence package meets minimum requirements.
    /// Must include at least a description and some supporting documents.
    pub fn is_valid(&self) -> bool {
        !self.description.trim().is_empty() && !self.supporting_documents.is_empty()
    }
}

// ---------------------------------------------------------------------------
// RepresentmentSubmission (entity within ChargebackCase)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepresentmentSubmission {
    pub submission_id: Uuid,
    pub evidence: RepresentmentEvidence,
    pub submitted_at: DateTime<Utc>,
    pub response_received_at: Option<DateTime<Utc>>,
    pub outcome: Option<ChargebackOutcome>,
}

// ---------------------------------------------------------------------------
// ChargebackCase aggregate
// ---------------------------------------------------------------------------

/// Core ChargebackCase aggregate root. Event-sourced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackCase {
    pub chargeback_id: Uuid,
    pub operator_id: Uuid,
    pub payment_intent_id: Uuid,
    pub acquirer_link_id: Uuid,
    pub status: ChargebackStatus,
    pub reason_code: String,
    pub amount_minor_units: i64,
    pub currency: String,
    pub received_at: DateTime<Utc>,
    pub representment_deadline: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub outcome: Option<ChargebackOutcome>,
    pub resolution_note: Option<String>,
    pub submissions: Vec<RepresentmentSubmission>,
}

impl ChargebackCase {
    /// Create a new chargeback case in `Received` status.
    /// INV-08: Must reference a captured PaymentIntent (enforced by caller).
    pub fn new(
        operator_id: Uuid,
        payment_intent_id: Uuid,
        acquirer_link_id: Uuid,
        reason_code: String,
        amount_minor_units: i64,
        currency: String,
    ) -> Result<Self, DisputeError> {
        if amount_minor_units <= 0 {
            return Err(DisputeError::InvalidAmount);
        }

        let now = Utc::now();
        // Default deadline: 30 days (Visa standard)
        let deadline = now + Duration::days(30);

        Ok(Self {
            chargeback_id: Uuid::now_v7(),
            operator_id,
            payment_intent_id,
            acquirer_link_id,
            status: ChargebackStatus::Received,
            reason_code,
            amount_minor_units,
            currency,
            received_at: now,
            representment_deadline: deadline,
            resolved_at: None,
            outcome: None,
            resolution_note: None,
            submissions: Vec::new(),
        })
    }

    /// Submit representment evidence.
    /// Preconditions: status must be Received or UnderReview.
    pub fn submit_representment(
        &mut self,
        evidence: RepresentmentEvidence,
    ) -> Result<&RepresentmentSubmission, DisputeError> {
        if self.status.is_resolved() {
            return Err(DisputeError::AlreadyResolved);
        }
        if self.status != ChargebackStatus::Received
            && self.status != ChargebackStatus::UnderReview
        {
            return Err(DisputeError::InvalidTransition);
        }
        if !evidence.is_valid() {
            return Err(DisputeError::InvalidRepresentmentEvidence);
        }
        if Utc::now() > self.representment_deadline {
            return Err(DisputeError::RepresentmentDeadlinePassed);
        }

        let submission = RepresentmentSubmission {
            submission_id: Uuid::now_v7(),
            evidence,
            submitted_at: Utc::now(),
            response_received_at: None,
            outcome: None,
        };

        self.status = ChargebackStatus::RepresentmentSubmitted;
        self.submissions.push(submission);
        Ok(self.submissions.last().unwrap())
    }

    /// Resolve the chargeback with an outcome.
    pub fn resolve(
        &mut self,
        outcome: ChargebackOutcome,
        resolution_note: Option<String>,
    ) -> Result<(), DisputeError> {
        if self.status.is_resolved() {
            return Err(DisputeError::AlreadyResolved);
        }

        let new_status = match outcome {
            ChargebackOutcome::Won => ChargebackStatus::Won,
            ChargebackOutcome::Lost => ChargebackStatus::Lost,
            ChargebackOutcome::Accepted => ChargebackStatus::Accepted,
            ChargebackOutcome::Escalated => ChargebackStatus::Escalated,
        };

        if !self.status.can_transition_to(&new_status) {
            return Err(DisputeError::InvalidTransition);
        }

        self.status = new_status;
        self.resolved_at = Some(Utc::now());
        self.resolution_note = resolution_note;

        // Update the latest submission outcome if any
        if let Some(submission) = self.submissions.last_mut() {
            submission.outcome = Some(outcome.clone());
            submission.response_received_at = Some(Utc::now());
            self.outcome = submission.outcome.clone();
        } else {
            self.outcome = Some(outcome);
        }

        Ok(())
    }

    /// Set representment deadline (scheme-specific).
    pub fn set_deadline(&mut self, days: i64) {
        self.representment_deadline = self.received_at + Duration::days(days);
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, thiserror::Error)]
pub enum DisputeError {
    #[error("Chargeback case not found: {0}")]
    NotFound(Uuid),
    #[error("Chargeback already resolved")]
    AlreadyResolved,
    #[error("Payment intent must be in captured state")]
    PaymentIntentNotCaptured,
    #[error("Invalid status transition")]
    InvalidTransition,
    #[error("Invalid chargeback amount")]
    InvalidAmount,
    #[error("Invalid representment evidence: missing required fields")]
    InvalidRepresentmentEvidence,
    #[error("Representment deadline has passed")]
    RepresentmentDeadlinePassed,
    #[error("Database error: {0}")]
    DatabaseError(String),
}
