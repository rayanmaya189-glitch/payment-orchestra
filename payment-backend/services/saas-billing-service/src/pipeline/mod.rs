//! Pipeline module for SaaS Billing service.
//!
//! This module orchestrates the command and query handlers,
//! providing a unified interface for the service.

use std::sync::Arc;

use crate::commands::{CommandHandler, SaasBillingCommandHandler};
use crate::commands::types::CreateTenantSubscriptionCommand;
use crate::queries::{QueryHandler, SaasBillingQueryHandler};
use crate::repository::InMemorySaasBillingRepository;

/// SaaS Billing pipeline that wires up all handlers.
pub struct SaasBillingPipeline {
    pub command_handler: Arc<dyn CommandHandler>,
    pub query_handler: Arc<dyn QueryHandler>,
}

impl SaasBillingPipeline {
    /// Create a new pipeline with in-memory repositories.
    pub async fn new_in_memory() -> Self {
        let repo = InMemorySaasBillingRepository::new();
        repo.seed_test_data().await;

        let command_handler = SaasBillingCommandHandler::new(
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
        );

        let query_handler = SaasBillingQueryHandler::new(
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
            repo.clone(),
        );

        Self {
            command_handler: Arc::new(command_handler),
            query_handler: Arc::new(query_handler),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_pipeline_creation() {
        let pipeline = SaasBillingPipeline::new_in_memory().await;
        
        // Test that we can get plans
        let plans = pipeline.query_handler.get_plans().await.unwrap();
        assert!(!plans.is_empty());
    }

    #[tokio::test]
    async fn test_create_subscription() {
        let pipeline = SaasBillingPipeline::new_in_memory().await;
        
        let operator_id = Uuid::now_v7();
        let plan_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        
        let subscription = pipeline.command_handler.create_subscription(
            CreateTenantSubscriptionCommand {
                operator_id,
                plan_id,
                created_by: operator_id,
                payment_method_id: None,
                trial_days: Some(14),
            }
        ).await.unwrap();
        
        assert_eq!(subscription.operator_id, operator_id);
        assert_eq!(subscription.plan_id, plan_id);
        assert_eq!(subscription.status, crate::domain::SubscriptionStatus::Trialing);
    }
}
