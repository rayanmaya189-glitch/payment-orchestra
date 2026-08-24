//! gRPC API implementation for SaaS Billing service.
//!
//! This module defines the gRPC service endpoints for managing:
//! - Subscription plans
//! - Tenant subscriptions
//! - Usage tracking
//! - Invoices
//! - Team members
//! - Audit logs

use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::commands::{CommandHandler, types::*};
use crate::queries::QueryHandler;

/// gRPC service implementation for SaaS Billing.
pub struct SaasBillingGrpcService<C, Q> {
    command_handler: Arc<C>,
    query_handler: Arc<Q>,
}

impl<C, Q> SaasBillingGrpcService<C, Q> {
    pub fn new(command_handler: Arc<C>, query_handler: Arc<Q>) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }
}

// Note: In production, this would implement the generated gRPC trait
// from platform-proto. For now, we provide the structure.

impl<C, Q> SaasBillingGrpcService<C, Q>
where
    C: CommandHandler + 'static,
    Q: QueryHandler + 'static,
{
    /// Get all active subscription plans.
    pub async fn get_plans(
        &self,
        _request: Request<()>,
    ) -> Result<Response<Vec<crate::domain::SaasPlan>>, Status> {
        let plans = self.query_handler.get_plans().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(plans))
    }

    /// Get a specific subscription plan by ID.
    pub async fn get_plan(
        &self,
        request: Request<String>,
    ) -> Result<Response<Option<crate::domain::SaasPlan>>, Status> {
        let plan_id_str = request.into_inner();
        let plan_id = uuid::Uuid::parse_str(&plan_id_str)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        
        let plan = self.query_handler.get_plan(plan_id).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(plan))
    }

    /// Create a new tenant subscription.
    pub async fn create_subscription(
        &self,
        request: Request<CreateTenantSubscriptionCommand>,
    ) -> Result<Response<crate::domain::TenantSubscription>, Status> {
        let cmd = request.into_inner();
        let subscription = self.command_handler.create_subscription(cmd).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(subscription))
    }

    /// Get tenant subscription.
    pub async fn get_subscription(
        &self,
        request: Request<String>,
    ) -> Result<Response<Option<crate::domain::TenantSubscription>>, Status> {
        let operator_id_str = request.into_inner();
        let operator_id = uuid::Uuid::parse_str(&operator_id_str)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        
        let subscription = self.query_handler.get_subscription(GetSubscriptionQuery {
            operator_id,
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(subscription))
    }

    /// Cancel tenant subscription.
    pub async fn cancel_subscription(
        &self,
        request: Request<CancelTenantSubscriptionCommand>,
    ) -> Result<Response<crate::domain::TenantSubscription>, Status> {
        let cmd = request.into_inner();
        let subscription = self.command_handler.cancel_subscription(cmd).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(subscription))
    }

    /// Get current usage for a tenant.
    pub async fn get_current_usage(
        &self,
        request: Request<String>,
    ) -> Result<Response<Option<crate::domain::TenantUsage>>, Status> {
        let operator_id_str = request.into_inner();
        let operator_id = uuid::Uuid::parse_str(&operator_id_str)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        
        let usage = self.query_handler.get_current_usage(GetCurrentUsageQuery {
            operator_id,
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(usage))
    }

    /// Record a transaction usage.
    pub async fn record_transaction_usage(
        &self,
        request: Request<RecordTransactionUsageCommand>,
    ) -> Result<Response<crate::domain::TenantUsage>, Status> {
        let cmd = request.into_inner();
        let usage = self.command_handler.record_transaction_usage(cmd).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(usage))
    }

    /// Get invoices for a tenant.
    pub async fn get_invoices(
        &self,
        request: Request<String>,
    ) -> Result<Response<Vec<crate::domain::SaasInvoice>>, Status> {
        let operator_id_str = request.into_inner();
        let operator_id = uuid::Uuid::parse_str(&operator_id_str)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        
        let invoices = self.query_handler.get_invoices(GetInvoicesQuery {
            operator_id,
            limit: Some(50),
            offset: Some(0),
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(invoices))
    }

    /// Create an invoice.
    pub async fn create_invoice(
        &self,
        request: Request<CreateInvoiceCommand>,
    ) -> Result<Response<crate::domain::SaasInvoice>, Status> {
        let cmd = request.into_inner();
        let invoice = self.command_handler.create_invoice(cmd).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(invoice))
    }

    /// Get team members for a tenant.
    pub async fn get_team_members(
        &self,
        request: Request<String>,
    ) -> Result<Response<Vec<crate::domain::TenantTeamMember>>, Status> {
        let operator_id_str = request.into_inner();
        let operator_id = uuid::Uuid::parse_str(&operator_id_str)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        
        let members = self.query_handler.get_team_members(GetTeamMembersQuery {
            operator_id,
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(members))
    }

    /// Invite a team member.
    pub async fn invite_team_member(
        &self,
        request: Request<InviteTeamMemberCommand>,
    ) -> Result<Response<crate::domain::TenantTeamMember>, Status> {
        let cmd = request.into_inner();
        let member = self.command_handler.invite_team_member(cmd).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(member))
    }

    /// Get audit logs for a tenant.
    pub async fn get_audit_logs(
        &self,
        request: Request<String>,
    ) -> Result<Response<Vec<crate::domain::AuditLog>>, Status> {
        let operator_id_str = request.into_inner();
        let operator_id = uuid::Uuid::parse_str(&operator_id_str)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        
        let logs = self.query_handler.get_audit_logs(GetAuditLogsQuery {
            operator_id,
            limit: Some(100),
            offset: Some(0),
            action_filter: None,
        }).await.map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(logs))
    }
}
