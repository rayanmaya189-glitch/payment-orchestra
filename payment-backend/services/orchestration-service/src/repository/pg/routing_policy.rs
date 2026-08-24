//! PostgreSQL-backed RoutingPolicyRepository using SeaORM CRUD.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::routing_policy::{
    ActiveModel as RoutingPolicyActiveModel, Column as RoutingPolicyColumn,
    Entity as RoutingPolicyEntity, Model as RoutingPolicyModel,
};
use super::PostgresOrchestrationRepository;
use crate::repository::RoutingPolicyRepository;

#[async_trait]
impl RoutingPolicyRepository for PostgresOrchestrationRepository {
    async fn load_active_routing_policy(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let result = RoutingPolicyEntity::find()
            .filter(RoutingPolicyColumn::OperatorId.eq(operator_id))
            .filter(RoutingPolicyColumn::Status.eq("Active"))
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save_routing_policy(&self, policy: &RoutingPolicy) -> Result<(), OrchestrationError> {
        let rules_json = serde_json::to_value(&policy.rules)
            .map_err(|e| OrchestrationError::DatabaseError(format!("Serialize rules: {}", e)))?;


        let model = RoutingPolicyModel {
            routing_policy_id: policy.routing_policy_id,
            operator_id: policy.operator_id,
            name: format!("routing_policy_{}", policy.routing_policy_id),
            status: match policy.status {
                PolicyStatus::Active => "Active".to_string(),
                PolicyStatus::Inactive => "Inactive".to_string(),
            },
            rules: rules_json,
            created_at: policy.created_at,
            activated_at: policy.activated_at,
        };

        let exists = RoutingPolicyEntity::find_by_id(policy.routing_policy_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            RoutingPolicyEntity::update(RoutingPolicyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            RoutingPolicyEntity::insert(RoutingPolicyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn load_routing_policy(&self, id: Uuid) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        let result = RoutingPolicyEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }
}

fn model_to_domain(m: RoutingPolicyModel) -> Result<RoutingPolicy, OrchestrationError> {
    let rules: Vec<RoutingRule> = serde_json::from_value(m.rules)
        .unwrap_or_default();

    Ok(RoutingPolicy {
        routing_policy_id: m.routing_policy_id,
        operator_id: m.operator_id,
        version: 1,
        status: match m.status.as_str() {
            "Active" => PolicyStatus::Active,
            _ => PolicyStatus::Inactive,
        },
        rules,
        failover_config: FailoverConfig::default(),
        partial_auth_strategy: PartialAuthStrategy::AcceptPartial,
        rotation_strategy: RotationStrategy::Priority,
        max_transaction_amount_minor: None,
        created_at: m.created_at,
        activated_at: m.activated_at,
    })
}
