//! RoutingPolicyRepository implementation for InMemoryOrchestrationRepository.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use super::in_memory::InMemoryOrchestrationRepository;
use super::traits::RoutingPolicyRepository;

#[async_trait]
impl RoutingPolicyRepository for InMemoryOrchestrationRepository {
    async fn load_active_routing_policy(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let active = self.active_policies.read().await;
        if let Some(policy_id) = active.get(&operator_id) {
            let store = self.routing_policies.read().await;
            return Ok(store.get(policy_id).cloned());
        }
        Ok(None)
    }

    async fn save_routing_policy(&self, policy: &RoutingPolicy) -> Result<(), OrchestrationError> {
        let mut store = self.routing_policies.write().await;
        store.insert(policy.routing_policy_id, policy.clone());
        if policy.status == PolicyStatus::Active {
            let mut active = self.active_policies.write().await;
            active.insert(policy.operator_id, policy.routing_policy_id);
        }
        Ok(())
    }

    async fn load_routing_policy(&self, id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let store = self.routing_policies.read().await;
        Ok(store.get(&id).cloned())
    }
}
