#![allow(clippy::too_many_lines)]
//! Domain model for BC-01 Operator Management.
//! Owns Operator aggregate and OperatorMember entity.

#[cfg(test)]
pub(crate) mod tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Operator aggregate root — represents a merchant tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub status: OperatorStatus,
    pub subdomain: String,
    pub email: String,
    pub verification_token_hash: Option<String>,
    pub provisioned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Operator lifecycle status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorStatus {
    Pending,
    ActiveUnverified,
    ActiveVerified,
    Suspended,
    ExpiredUnverified,
}

impl OperatorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::ActiveUnverified => "active_unverified",
            Self::ActiveVerified => "active_verified",
            Self::Suspended => "suspended",
            Self::ExpiredUnverified => "expired_unverified",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "active_unverified" => Some(Self::ActiveUnverified),
            "active_verified" => Some(Self::ActiveVerified),
            "suspended" => Some(Self::Suspended),
            "expired_unverified" => Some(Self::ExpiredUnverified),
            _ => None,
        }
    }

    /// Valid state transitions
    pub fn can_transition_to(&self, target: &OperatorStatus) -> bool {
        matches!(
            (self, target),
            (Self::Pending, Self::ActiveUnverified)
                | (Self::ActiveUnverified, Self::ActiveVerified)
                | (Self::ActiveUnverified, Self::ExpiredUnverified)
                | (Self::ActiveVerified, Self::Suspended)
                | (Self::Suspended, Self::ActiveVerified) // unsuspend
        )
    }

    #[allow(dead_code)]
    pub fn can_process_live_transactions(&self) -> bool {
        matches!(self, Self::ActiveVerified)
    }

    #[allow(dead_code)]
    pub fn can_access_sandbox(&self) -> bool {
        !matches!(self, Self::Pending | Self::Suspended)
    }
}

impl Operator {
    pub fn new(
        id: Uuid,
        legal_name: String,
        trade_license_no: String,
        country: String,
        email: String,
        subdomain: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            legal_name,
            trade_license_no,
            country,
            status: OperatorStatus::Pending,
            subdomain,
            email,
            verification_token_hash: None,
            provisioned_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn verify_email(&mut self) -> Result<(), OperatorError> {
        if !self.status.can_transition_to(&OperatorStatus::ActiveUnverified) {
            return Err(OperatorError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: "active_unverified".to_string(),
            });
        }
        self.status = OperatorStatus::ActiveUnverified;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_status(&mut self, new_status: OperatorStatus) -> Result<(), OperatorError> {
        if !self.status.can_transition_to(&new_status) {
            return Err(OperatorError::InvalidStatusTransition {
                from: self.status.as_str().to_string(),
                to: new_status.as_str().to_string(),
            });
        }
        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn mark_provisioned(&mut self) {
        self.provisioned_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

// ─── Error Types ──────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum OperatorError {
    #[error("Operator not found: {0}")]
    NotFound(Uuid),

    #[error("Duplicate trade license: {0}")]
    DuplicateTradeLicense(String),

    #[error("Duplicate subdomain: {0}")]
    DuplicateSubdomain(String),

    #[error("Invalid trade license format: {0}")]
    InvalidTradeLicenseFormat(String),

    #[error("Invalid status transition: {from} -> {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("Email verification failed: {0}")]
    EmailVerificationFailed(String),

    #[error("Operator suspended")]
    Suspended,

    #[error("KYB not approved")]
    KybNotApproved,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl From<OperatorError> for platform_error::PlatformError {
    fn from(e: OperatorError) -> Self {
        match e {
            OperatorError::NotFound(id) => {
                platform_error::PlatformError::NotFound { resource: "operator", id }
            }
            OperatorError::DuplicateTradeLicense(_)
            | OperatorError::DuplicateSubdomain(_) => {
                platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::IdempotencyKeyConflict,
                )
            }
            OperatorError::InvalidTradeLicenseFormat(ref msg) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "trade_license_no".into(),
                        reason: msg.clone(),
                    },
                )
            }
            OperatorError::InvalidStatusTransition { from, to } => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidStateTransition {
                        from_state: from,
                        command: to,
                    },
                )
            }
            OperatorError::EmailVerificationFailed(msg) => {
                platform_error::PlatformError::Validation(
                    platform_error::ValidationError::InvalidValue {
                        field: "verification_token".into(),
                        reason: msg,
                    },
                )
            }
            OperatorError::Suspended => {
                platform_error::PlatformError::AuthorizationDenied(
                    "Operator account suspended".into(),
                )
            }
            OperatorError::KybNotApproved => {
                platform_error::PlatformError::AuthorizationDenied(
                    "KYB not yet approved".into(),
                )
            }
            OperatorError::DatabaseError(ref msg) => {
                platform_error::PlatformError::Internal(msg.clone())
            }
        }
    }
}

