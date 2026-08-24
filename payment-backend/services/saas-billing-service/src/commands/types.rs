//! Command types for SaaS Billing service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Subscription Commands ───────────────────────────────────────────────────

/// Command to create a new tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantSubscriptionCommand {
    pub operator_id: Uuid,
    pub plan_id: Uuid,
    pub created_by: Uuid,
    pub payment_method_id: Option<String>,
    pub trial_days: Option<i32>,
}

/// Command to update a tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTenantSubscriptionCommand {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub plan_id: Option<Uuid>,
    pub payment_method_id: Option<String>,
}

/// Command to cancel a tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTenantSubscriptionCommand {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
    pub reason: Option<String>,
}

/// Command to activate a tenant subscription (convert trial to active).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateTenantSubscriptionCommand {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
}

/// Command to pause a tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseTenantSubscriptionCommand {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
}

/// Command to resume a tenant subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeTenantSubscriptionCommand {
    pub subscription_id: Uuid,
    pub operator_id: Uuid,
}

// ─── Usage Commands ──────────────────────────────────────────────────────────

/// Command to record a transaction usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordTransactionUsageCommand {
    pub operator_id: Uuid,
    pub amount_minor: i64,
}

/// Command to record an API call usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordApiCallUsageCommand {
    pub operator_id: Uuid,
}

/// Command to record storage usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordStorageUsageCommand {
    pub operator_id: Uuid,
    pub bytes: i64,
}

/// Command to record an AI query usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordAiQueryUsageCommand {
    pub operator_id: Uuid,
}

// ─── Invoice Commands ────────────────────────────────────────────────────────

/// Command to create an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceCommand {
    pub operator_id: Uuid,
    pub subscription_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Command to finalize an invoice (convert draft to open).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeInvoiceCommand {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
}

/// Command to mark an invoice as paid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkInvoicePaidCommand {
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub stripe_invoice_id: Option<String>,
}

// ─── Team Commands ───────────────────────────────────────────────────────────

/// Command to invite a team member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteTeamMemberCommand {
    pub operator_id: Uuid,
    pub email: String,
    pub role: String,
    pub invited_by: Uuid,
}

/// Command to accept a team member invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptTeamMemberCommand {
    pub membership_id: Uuid,
    pub principal_id: Uuid,
}

/// Command to suspend a team member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendTeamMemberCommand {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
}

/// Command to change a team member's role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTeamMemberRoleCommand {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub new_role: String,
}

// ─── Audit Commands ──────────────────────────────────────────────────────────

/// Command to create an audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogCommand {
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ─── Query Commands ──────────────────────────────────────────────────────────

/// Command to get subscription details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSubscriptionQuery {
    pub operator_id: Uuid,
}

/// Command to get usage for current period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCurrentUsageQuery {
    pub operator_id: Uuid,
}

/// Command to get invoices for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInvoicesQuery {
    pub operator_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Command to get team members for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTeamMembersQuery {
    pub operator_id: Uuid,
}

/// Command to get audit logs for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAuditLogsQuery {
    pub operator_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub action_filter: Option<String>,
}
