//! Event types for SaaS Billing service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SaaS Billing domain events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SaaSbillingEvent {
    // Subscription events
    SubscriptionCreated(SubscriptionCreated),
    SubscriptionActivated(SubscriptionActivated),
    SubscriptionCanceled(SubscriptionCanceled),
    SubscriptionRenewed(SubscriptionRenewed),
    SubscriptionUpdated(SubscriptionUpdated),

    // Usage events
    UsageRecorded(UsageRecorded),

    // Invoice events
    InvoiceCreated(InvoiceCreated),
    InvoiceFinalized(InvoiceFinalized),
    InvoicePaid(InvoicePaid),
    InvoiceVoided(InvoiceVoided),

    // Team events
    TeamMemberInvited(TeamMemberInvited),
    TeamMemberAccepted(TeamMemberAccepted),
    TeamMemberSuspended(TeamMemberSuspended),
    TeamMemberRoleChanged(TeamMemberRoleChanged),

    // Audit events
    AuditLogCreated(AuditLogCreated),
}

// ─── Subscription Events ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionCreated {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub plan_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionActivated {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub activated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionCanceled {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub reason: Option<String>,
    pub canceled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRenewed {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub new_period_end: DateTime<Utc>,
    pub renewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionUpdated {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub old_plan_id: Option<Uuid>,
    pub new_plan_id: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

// ─── Usage Events ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecorded {
    pub operator_id: Uuid,
    pub usage_type: String,
    pub amount: i64,
    pub recorded_at: DateTime<Utc>,
}

// ─── Invoice Events ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceCreated {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub subscription_id: Uuid,
    pub total_minor: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceFinalized {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub finalized_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePaid {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub paid_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceVoided {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub voided_at: DateTime<Utc>,
}

// ─── Team Events ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberInvited {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub email: String,
    pub role: String,
    pub invited_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberAccepted {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberSuspended {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub suspended_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberRoleChanged {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub old_role: String,
    pub new_role: String,
    pub changed_at: DateTime<Utc>,
}

// ─── Audit Events ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogCreated {
    pub log_id: Uuid,
    pub operator_id: Uuid,
    pub action: String,
    pub resource: String,
    pub created_at: DateTime<Utc>,
}

// ─── Event Type Constants ────────────────────────────────────────────────────

impl SaaSbillingEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SubscriptionCreated(_) => "subscription.created",
            Self::SubscriptionActivated(_) => "subscription.activated",
            Self::SubscriptionCanceled(_) => "subscription.canceled",
            Self::SubscriptionRenewed(_) => "subscription.renewed",
            Self::SubscriptionUpdated(_) => "subscription.updated",
            Self::UsageRecorded(_) => "usage.recorded",
            Self::InvoiceCreated(_) => "invoice.created",
            Self::InvoiceFinalized(_) => "invoice.finalized",
            Self::InvoicePaid(_) => "invoice.paid",
            Self::InvoiceVoided(_) => "invoice.voided",
            Self::TeamMemberInvited(_) => "team_member.invited",
            Self::TeamMemberAccepted(_) => "team_member.accepted",
            Self::TeamMemberSuspended(_) => "team_member.suspended",
            Self::TeamMemberRoleChanged(_) => "team_member.role_changed",
            Self::AuditLogCreated(_) => "audit_log.created",
        }
    }

    pub fn operator_id(&self) -> Uuid {
        match self {
            Self::SubscriptionCreated(e) => e.operator_id,
            Self::SubscriptionActivated(e) => e.operator_id,
            Self::SubscriptionCanceled(e) => e.operator_id,
            Self::SubscriptionRenewed(e) => e.operator_id,
            Self::SubscriptionUpdated(e) => e.operator_id,
            Self::UsageRecorded(e) => e.operator_id,
            Self::InvoiceCreated(e) => e.operator_id,
            Self::InvoiceFinalized(e) => e.operator_id,
            Self::InvoicePaid(e) => e.operator_id,
            Self::InvoiceVoided(e) => e.operator_id,
            Self::TeamMemberInvited(e) => e.operator_id,
            Self::TeamMemberAccepted(e) => e.operator_id,
            Self::TeamMemberSuspended(e) => e.operator_id,
            Self::TeamMemberRoleChanged(e) => e.operator_id,
            Self::AuditLogCreated(e) => e.operator_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type() {
        let event = SaaSbillingEvent::SubscriptionCreated(SubscriptionCreated {
            subscription_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            plan_id: Uuid::now_v7(),
            status: "active".into(),
            created_at: Utc::now(),
        });
        assert_eq!(event.event_type(), "subscription.created");
    }

    #[test]
    fn test_event_operator_id() {
        let operator_id = Uuid::now_v7();
        let event = SaaSbillingEvent::UsageRecorded(UsageRecorded {
            operator_id,
            usage_type: "transaction".into(),
            amount: 1000,
            recorded_at: Utc::now(),
        });
        assert_eq!(event.operator_id(), operator_id);
    }
}
