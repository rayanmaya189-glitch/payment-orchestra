//! Routing policy command handlers.

use std::hash::{Hash, Hasher};
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use super::types::*;
use super::OrchestrationCommandHandler;

impl<R: OrchestrationRepository + Send + Sync> OrchestrationCommandHandler<R> {
    pub(crate) async fn activate_routing_policy_impl(&self, cmd: ActivateRoutingPolicy) -> Result<RoutingPolicyResult, OrchestrationError> {
        let policy_id = Uuid::now_v7();

        if let Some(mut existing) = self.repo.load_active_routing_policy(cmd.operator_id).await? {
            existing.status = PolicyStatus::Inactive;
            self.repo.save_routing_policy(&existing).await?;
        }

        let mut policy = RoutingPolicy::new(policy_id, cmd.operator_id, cmd.rules);
        policy.failover_config = cmd.failover_config;
        policy.partial_auth_strategy = cmd.partial_auth_strategy;
        policy.rotation_strategy = cmd.rotation_strategy;
        policy.max_transaction_amount_minor = cmd.max_transaction_amount_minor;
        policy.status = PolicyStatus::Active;
        policy.activated_at = Some(Utc::now());

        let rules_str = format!("{:?}", policy.rules);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        rules_str.hash(&mut hasher);
        let rules_hash = format!("{:x}", hasher.finish());

        let event = PaymentEvent::RoutingPolicyActivated(RoutingPolicyActivated {
            routing_policy_id: policy_id,
            operator_id: cmd.operator_id,
            version: policy.version,
            rules_hash,
            occurred_at: Utc::now(),
        });

        self.repo.save_routing_policy(&policy).await?;

        Ok(RoutingPolicyResult {
            routing_policy_id: policy_id,
            version: policy.version,
            status: PolicyStatus::Active,
            event,
        })
    }
}
