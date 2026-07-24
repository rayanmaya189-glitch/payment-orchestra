//! PostgreSQL-backed orchestration repositories using SeaORM + platform-db entities.
//!
//! Implements 3 persistent traits (PaymentIntentRepository, RoutingPolicyRepository,
//! PaymentMethodTokenRepository) and 2 in-memory traits (IdempotencyCache, AcquirerLinkProvider).

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::*;
use crate::domain::*;
use crate::entities::payment_intent::{
    Entity as PaymentIntentEntity,
    ActiveModel as PaymentIntentActiveModel,
    Model as PaymentIntentModel,
    Column as PaymentIntentColumn,
};
use crate::entities::routing_policy::{
    Entity as RoutingPolicyEntity,
    ActiveModel as RoutingPolicyActiveModel,
    Model as RoutingPolicyModel,
    Column as RoutingPolicyColumn,
};
use crate::entities::payment_method_token::{
    Entity as PaymentMethodTokenEntity,
    ActiveModel as PaymentMethodTokenActiveModel,
    Model as PaymentMethodTokenModel,
    Column as PaymentMethodTokenColumn,
};

/// Combined PostgreSQL-backed orchestration repository.
///
/// Uses SeaORM for persistent entities and in-memory maps for
/// idempotency cache and acquirer link provider (no dedicated DB tables yet).
pub struct PostgresOrchestrationRepository {
    pub db: DatabaseConnection,
    idempotency_cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    active_policies: Arc<RwLock<HashMap<Uuid, Uuid>>>, // operator_id → policy_id
    active_links: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // operator_id → [link_ids]
}

impl PostgresOrchestrationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            idempotency_cache: Arc::new(RwLock::new(HashMap::new())),
            active_policies: Arc::new(RwLock::new(HashMap::new())),
            active_links: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Seed active acquirer links for testing.
    pub async fn set_active_links(&self, operator_id: Uuid, link_ids: Vec<Uuid>) {
        let mut links = self.active_links.write().await;
        links.insert(operator_id, link_ids);
    }
}

// ─── PaymentIntentRepository ─────────────────────────────────────────────────

#[async_trait]
impl PaymentIntentRepository for PostgresOrchestrationRepository {
    async fn load_payment_intent(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentIntent>, OrchestrationError> {
        let result = PaymentIntentEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(payment_intent_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_payment_intent(
        &self,
        intent: &PaymentIntent,
    ) -> Result<(), OrchestrationError> {
        let model = payment_intent_domain_to_model(intent)?;
        let exists = PaymentIntentEntity::find_by_id(intent.payment_intent_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            PaymentIntentEntity::update(PaymentIntentActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            PaymentIntentEntity::insert(PaymentIntentActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn list_payment_intents_for_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<PaymentIntent>, OrchestrationError> {
        let models = PaymentIntentEntity::find()
            .filter(PaymentIntentColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        models.into_iter().map(payment_intent_model_to_domain).collect()
    }
}

// ─── RoutingPolicyRepository ─────────────────────────────────────────────────

#[async_trait]
impl RoutingPolicyRepository for PostgresOrchestrationRepository {
    async fn load_active_routing_policy(
        &self,
        operator_id: Uuid,
    ) -> Result<Option<RoutingPolicy>, OrchestrationError> {
        // First check in-memory active policy index
        {
            let active = self.active_policies.read().await;
            if let Some(policy_id) = active.get(&operator_id) {
                let store = self.load_routing_policy(*policy_id).await?;
                if store.is_some() {
                    return Ok(store);
                }
            }
        }

        // Fall back to DB query: find policy with status = "active" for this operator
        let result = RoutingPolicyEntity::find()
            .filter(RoutingPolicyColumn::OperatorId.eq(operator_id))
            .filter(RoutingPolicyColumn::Status.eq("active"))
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        match result {
            Some(m) => {
                let policy = routing_policy_model_to_domain(m)?;
                // Cache in-memory
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

        // Update in-memory active policy index
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

// ─── PaymentMethodTokenRepository ────────────────────────────────────────────

#[async_trait]
impl PaymentMethodTokenRepository for PostgresOrchestrationRepository {
    async fn load_payment_method_token(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        let result = PaymentMethodTokenEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(payment_method_token_model_to_domain(m))),
            None => Ok(None),
        }
    }

    async fn save_payment_method_token(
        &self,
        token: &PaymentMethodToken,
    ) -> Result<(), OrchestrationError> {
        let model = payment_method_token_domain_to_model(token);
        let exists = PaymentMethodTokenEntity::find_by_id(token.token_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            PaymentMethodTokenEntity::update(PaymentMethodTokenActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            PaymentMethodTokenEntity::insert(PaymentMethodTokenActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_active_tokens_for_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        let models = PaymentMethodTokenEntity::find()
            .filter(PaymentMethodTokenColumn::OperatorId.eq(operator_id))
            .filter(PaymentMethodTokenColumn::TokenStatus.eq("Active"))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        Ok(models.into_iter().map(payment_method_token_model_to_domain).collect())
    }
}

// ─── IdempotencyCache (In-Memory) ────────────────────────────────────────────

#[async_trait]
impl IdempotencyCache for PostgresOrchestrationRepository {
    async fn check_idempotency(
        &self,
        key: &str,
    ) -> Result<IdempotencyResult, OrchestrationError> {
        let cache = self.idempotency_cache.read().await;
        match cache.get(key) {
            Some(result) => Ok(IdempotencyResult::Duplicate(result.clone())),
            None => Ok(IdempotencyResult::New),
        }
    }

    async fn store_idempotency(
        &self,
        key: &str,
        result: &serde_json::Value,
    ) -> Result<(), OrchestrationError> {
        let mut cache = self.idempotency_cache.write().await;
        cache.insert(key.to_string(), result.clone());
        Ok(())
    }
}

// ─── AcquirerLinkProvider (In-Memory) ────────────────────────────────────────

#[async_trait]
impl AcquirerLinkProvider for PostgresOrchestrationRepository {
    async fn list_active_acquirer_links(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<Uuid>, OrchestrationError> {
        let links = self.active_links.read().await;
        Ok(links.get(&operator_id).cloned().unwrap_or_default())
    }
}

// ─── Domain ↔ Model conversion: PaymentIntent ────────────────────────────────

fn payment_intent_domain_to_model(
    pi: &PaymentIntent,
) -> Result<PaymentIntentModel, OrchestrationError> {
    Ok(PaymentIntentModel {
        payment_intent_id: pi.payment_intent_id,
        operator_id: pi.operator_id,
        amount_minor_units: pi.requested_amount.amount_minor_units,
        currency: pi.currency.clone(),
        status: format!("{:?}", pi.status),
        payment_method_type: pi.source_type.clone().unwrap_or_default(),
        captured_amount_minor: pi.captured_amount.amount_minor_units,
        refunded_amount_minor: pi.refunded_amount.amount_minor_units,
        routing_attempts: serde_json::to_value(&pi.routing_attempts)
            .map_err(|e| OrchestrationError::Validation(format!("Serialize routing_attempts: {}", e)))?,
        metadata_json: pi.metadata.as_ref().map(|m| m.to_string()),
        error_message: None,
        created_at: pi.created_at,
        updated_at: pi.updated_at,
    })
}

fn payment_intent_model_to_domain(
    m: PaymentIntentModel,
) -> Result<PaymentIntent, OrchestrationError> {
    let status = match m.status.as_str() {
        "Created" => PaymentStatus::Created,
        "Authorizing" => PaymentStatus::Authorizing,
        "Authorized" => PaymentStatus::Authorized,
        "Capturing" => PaymentStatus::Capturing,
        "Captured" => PaymentStatus::Captured,
        "PartiallyCaptured" => PaymentStatus::PartiallyCaptured,
        "Voided" => PaymentStatus::Voided,
        "AuthorizationExpired" => PaymentStatus::AuthorizationExpired,
        "Failed" => PaymentStatus::Failed,
        "FailedAllRoutes" => PaymentStatus::FailedAllRoutes,
        "Refunding" => PaymentStatus::Refunding,
        "Refunded" => PaymentStatus::Refunded,
        "PartiallyRefunded" => PaymentStatus::PartiallyRefunded,
        _ => return Err(OrchestrationError::Validation(format!("Unknown status: {}", m.status))),
    };

    let routing_attempts: Vec<RoutingAttempt> = serde_json::from_value(m.routing_attempts)
        .map_err(|e| OrchestrationError::Validation(format!("Deserialize routing_attempts: {}", e)))?;

    let metadata: Option<serde_json::Value> = m.metadata_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(PaymentIntent {
        payment_intent_id: m.payment_intent_id,
        operator_id: m.operator_id,
        status,
        requested_amount: Money {
            amount_minor_units: m.amount_minor_units,
            currency: m.currency.clone(),
        },
        authorized_amount: Money::zero(&m.currency),
        captured_amount: Money {
            amount_minor_units: m.captured_amount_minor,
            currency: m.currency.clone(),
        },
        refunded_amount: Money {
            amount_minor_units: m.refunded_amount_minor,
            currency: m.currency.clone(),
        },
        currency: m.currency,
        idempotency_key: String::new(),
        payment_method_token_id: None,
        routing_policy_id: None,
        deployment_epoch: 0,
        purpose: PaymentPurpose::Payment,
        metadata,
        source_type: Some(m.payment_method_type),
        source_id: None,
        risk_score: None,
        risk_level: None,
        expected_settlement_date: None,
        settlement_cycle: None,
        gateway_profile_id: None,
        gateway_profile_version: None,
        gateway_rotation_strategy: None,
        gateway_selection_reason: None,
        routing_attempts,
        version: 0,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
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

// ─── Domain ↔ Model conversion: PaymentMethodToken ───────────────────────────

fn payment_method_token_domain_to_model(
    t: &PaymentMethodToken,
) -> PaymentMethodTokenModel {
    PaymentMethodTokenModel {
        token_id: t.token_id,
        operator_id: t.operator_id,
        token_status: format!("{:?}", t.token_status),
        payment_method_type: t.payment_method_type.clone(),
        token_ref: t.acquirer_token_reference.clone(),
        created_at: t.created_at,
        expires_at: t.expires_at,
    }
}

fn payment_method_token_model_to_domain(
    m: PaymentMethodTokenModel,
) -> PaymentMethodToken {
    let token_status = match m.token_status.as_str() {
        "Active" => TokenStatus::Active,
        "Expired" => TokenStatus::Expired,
        "Revoked" => TokenStatus::Revoked,
        _ => TokenStatus::Active,
    };

    PaymentMethodToken {
        token_id: m.token_id,
        operator_id: m.operator_id,
        payment_method_type: m.payment_method_type,
        last_four: String::new(),
        card_brand: None,
        expiry_month: None,
        expiry_year: None,
        token_status,
        acquirer_link_id: Uuid::default(),
        acquirer_token_reference: m.token_ref,
        encrypted_token: Vec::new(),
        created_at: m.created_at,
        expires_at: m.expires_at,
        revoked_at: None,
        revocation_reason: None,
    }
}
