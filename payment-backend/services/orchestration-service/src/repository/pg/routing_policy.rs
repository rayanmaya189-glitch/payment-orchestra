use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresOrchestrationRepository;
use crate::repository::RoutingPolicyRepository;
use crate::domain::*;
use crate::entities::routing_policy::{
    Entity as RoutingPolicyEntity,
    ActiveModel as RoutingPolicyActiveModel,
    Model as RoutingPolicyModel,
    Column as RoutingPolicyColumn,
};

#[async_trait]
impl RoutingPolicyRepository for PostgresOrchestrationRepository {
    async fn load_active_routing_policy(
        &self,
        operator_id: Uuid,
    ) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        {
            let active = self.active_policies.read().await;
            if let Some(policy_id) = active.get(&operator_id) {
                let store = self.load_routing_policy(*policy_id).await?;
                if store.is_some() {
                    return Ok(store);
                }
            }
        }

        let result = RoutingPolicyEntity::find()
            .filter(RoutingPolicyColumn::OperatorId.eq(operator_id))
            .filter(RoutingPolicyColumn::Status.eq("active"))
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        match result {
            Some(m) => {
                let policy = routing_policy_model_to_domain(m)?;
                self.active_policies
                    .write()
                    .await
                    .insert(operator_id, policy.routing_policy_id);
                Ok(Some(policy))
            }
            None => Ok(None),
        }
    }

    async fn save_routing_policy(
        &self,
        policy: &RoutingPolicy,
    ) -> Result<(), OrchestrationError> {
        let model = routing_policy_domain_to_model(policy)?;
        let exists = RoutingPolicyEntity::find_by_id(policy.routing_policy_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            RoutingPolicyEntity::update(RoutingPolicyActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            RoutingPolicyEntity::insert(RoutingPolicyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }

        if policy.status == PolicyStatus::Active {
            let mut active = self.active_policies.write().await;
            active.insert(policy.operator_id, policy.routing_policy_id);
        }

        Ok(())
    }

    async fn load_routing_policy(
        &self,
        id: Uuid,
    ) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let result = RoutingPolicyEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(routing_policy_model_to_domain(m)?)),
            None => Ok(None),
        }
    }
}

// ─── Domain ↔ Model conversion: RoutingPolicy ────────────────────────────────

fn routing_policy_domain_to_model(
    rp: &RoutingPolicy,
) -> Result<RoutingPolicyModel, OrchestrationError> {
    Ok(RoutingPolicyModel {
        routing_policy_id: rp.routing_policy_id,
        operator_id: rp.operator_id,
        name: format!("policy-v{}", rp.version),
        status: rp.status.to_string(),
        rules: serde_json::to_value(&rp.rules)
            .map_err(|e| OrchestrationError::Validation(format!("Serialize rules: {}", e)))?,
        created_at: rp.created_at,
        activated_at: rp.activated_at,
    })
}

fn routing_policy_model_to_domain(
    m: RoutingPolicyModel,
) -> Result<RoutingPolicy, OrchestrationError> {
    let status = match m.status.as_str() {
        "active" => PolicyStatus::Active,
        _ => PolicyStatus::Inactive,
    };

    let rules: Vec<RoutingRule> = serde_json::from_value(m.rules)
        .map_err(|e| OrchestrationError::Validation(format!("Deserialize rules: {}", e)))?;

    Ok(RoutingPolicy {
        routing_policy_id: m.routing_policy_id,
        operator_id: m.operator_id,
        version: 1,
        status,
        rules,
        failover_config: FailoverConfig::default(),
        partial_auth_strategy: PartialAuthStrategy::AcceptPartial,
        rotation_strategy: RotationStrategy::Priority,
        max_transaction_amount_minor: None,
        created_at: m.created_at,
        activated_at: m.activated_at,
    })
}
