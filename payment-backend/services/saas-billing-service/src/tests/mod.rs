//! Tests for SaaS Billing service.

use uuid::Uuid;
use chrono::{Datelike, Timelike};
use crate::pipeline::SaasBillingPipeline;
use crate::commands::types::*;
use crate::domain::*;

#[tokio::test]
async fn test_subscription_lifecycle() {
    let pipeline = SaasBillingPipeline::new_in_memory().await;
    
    let operator_id = Uuid::now_v7();
    let plan_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    
    // Create subscription with trial
    let subscription = pipeline.command_handler.create_subscription(
        CreateTenantSubscriptionCommand {
            operator_id,
            plan_id,
            created_by: operator_id,
            payment_method_id: None,
            trial_days: Some(14),
        }
    ).await.unwrap();
    
    assert_eq!(subscription.status, SubscriptionStatus::Trialing);
    assert!(subscription.trial_ends_at.is_some());
    
    // Activate subscription (convert trial to active)
    let activated = pipeline.command_handler.activate_subscription(
        ActivateTenantSubscriptionCommand {
            subscription_id: subscription.subscription_id,
            operator_id,
        }
    ).await.unwrap();
    
    assert_eq!(activated.status, SubscriptionStatus::Active);
    
    // Cancel subscription
    let canceled = pipeline.command_handler.cancel_subscription(
        CancelTenantSubscriptionCommand {
            subscription_id: activated.subscription_id,
            operator_id,
            reason: Some("No longer needed".into()),
        }
    ).await.unwrap();
    
    assert_eq!(canceled.status, SubscriptionStatus::Canceled);
    assert!(canceled.canceled_at.is_some());
    assert_eq!(canceled.cancel_reason, Some("No longer needed".into()));
}

#[tokio::test]
async fn test_usage_tracking() {
    let pipeline = SaasBillingPipeline::new_in_memory().await;
    
    let operator_id = Uuid::now_v7();
    
    // Record transactions
    let usage = pipeline.command_handler.record_transaction_usage(
        RecordTransactionUsageCommand {
            operator_id,
            amount_minor: 1000,
        }
    ).await.unwrap();
    
    assert_eq!(usage.transaction_count, 1);
    assert_eq!(usage.transaction_volume_minor, 1000);
    
    // Record more transactions
    let usage = pipeline.command_handler.record_transaction_usage(
        RecordTransactionUsageCommand {
            operator_id,
            amount_minor: 2000,
        }
    ).await.unwrap();
    
    assert_eq!(usage.transaction_count, 2);
    assert_eq!(usage.transaction_volume_minor, 3000);
    
    // Record API calls
    let usage = pipeline.command_handler.record_api_call_usage(
        RecordApiCallUsageCommand {
            operator_id,
        }
    ).await.unwrap();
    
    assert_eq!(usage.api_calls, 1);
}

#[tokio::test]
async fn test_invoice_creation() {
    let pipeline = SaasBillingPipeline::new_in_memory().await;
    
    let operator_id = Uuid::now_v7();
    let plan_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    
    // Create subscription
    let subscription = pipeline.command_handler.create_subscription(
        CreateTenantSubscriptionCommand {
            operator_id,
            plan_id,
            created_by: operator_id,
            payment_method_id: None,
            trial_days: None,
        }
    ).await.unwrap();
    
    // Record some usage
    for _ in 0..150 {
        pipeline.command_handler.record_transaction_usage(
            RecordTransactionUsageCommand {
                operator_id,
                amount_minor: 1000,
            }
        ).await.unwrap();
    }
    
    // Create invoice
    // Must match the period_start used by record_transaction_usage (truncated to 1st of month midnight)
    let now = chrono::Utc::now();
    let period_start = now.with_day(1).unwrap().with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let period_end = period_start + chrono::Duration::days(30);
    
    let invoice = pipeline.command_handler.create_invoice(
        CreateInvoiceCommand {
            operator_id,
            subscription_id: subscription.subscription_id,
            period_start,
            period_end,
        }
    ).await.unwrap();
    
    assert_eq!(invoice.status, InvoiceStatus::Draft);
    assert!(invoice.total_minor > 0);
    assert!(!invoice.line_items.is_empty());
}

#[tokio::test]
async fn test_team_management() {
    let pipeline = SaasBillingPipeline::new_in_memory().await;
    
    let operator_id = Uuid::now_v7();
    let owner_id = Uuid::now_v7();
    let member_id = Uuid::now_v7();
    
    // Invite team member
    let member = pipeline.command_handler.invite_team_member(
        InviteTeamMemberCommand {
            operator_id,
            email: "dev@example.com".into(),
            role: "developer".into(),
            invited_by: owner_id,
        }
    ).await.unwrap();
    
    assert_eq!(member.status, TeamMemberStatus::Pending);
    assert_eq!(member.role, "developer");
    
    // Accept invitation
    let accepted = pipeline.command_handler.accept_team_member(
        AcceptTeamMemberCommand {
            membership_id: member.membership_id,
            principal_id: member_id,
        }
    ).await.unwrap();
    
    assert_eq!(accepted.status, TeamMemberStatus::Active);
    assert!(accepted.accepted_at.is_some());
    
    // Change role
    let updated = pipeline.command_handler.change_team_member_role(
        ChangeTeamMemberRoleCommand {
            membership_id: member.membership_id,
            operator_id,
            new_role: "admin".into(),
        }
    ).await.unwrap();
    
    assert_eq!(updated.role, "admin");
}

#[tokio::test]
async fn test_audit_logging() {
    let pipeline = SaasBillingPipeline::new_in_memory().await;
    
    let operator_id = Uuid::now_v7();
    let principal_id = Uuid::now_v7();
    
    // Create audit log
    let log = pipeline.command_handler.create_audit_log(
        CreateAuditLogCommand {
            operator_id,
            principal_id,
            action: "login".into(),
            resource: "principal".into(),
            resource_id: Some(principal_id.to_string()),
            old_value: None,
            new_value: Some(serde_json::json!({"status": "active"})),
            ip_address: Some("192.168.1.1".into()),
            user_agent: Some("Mozilla/5.0".into()),
            metadata: None,
        }
    ).await.unwrap();
    
    assert_eq!(log.action, "login");
    assert_eq!(log.resource, "principal");
    assert!(log.ip_address.is_some());
}

#[tokio::test]
async fn test_duplicate_subscription_prevention() {
    let pipeline = SaasBillingPipeline::new_in_memory().await;
    
    let operator_id = Uuid::now_v7();
    let plan_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    
    // Create first subscription
    let _ = pipeline.command_handler.create_subscription(
        CreateTenantSubscriptionCommand {
            operator_id,
            plan_id,
            created_by: operator_id,
            payment_method_id: None,
            trial_days: None,
        }
    ).await.unwrap();
    
    // Try to create duplicate
    let result = pipeline.command_handler.create_subscription(
        CreateTenantSubscriptionCommand {
            operator_id,
            plan_id,
            created_by: operator_id,
            payment_method_id: None,
            trial_days: None,
        }
    ).await;
    
    assert!(result.is_err());
}
