pub mod event_store;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, ConnectionTrait, Statement};
use uuid::Uuid;

use crate::domain::aggregates::{PaymentIntent, PaymentIntentEvent, RoutingAttempt, RoutingPolicy};
use crate::infrastructure::entities::{payment_intent, routing_attempt, routing_policy};
use crate::infrastructure::repository::PaymentIntentRepository;
use platform_error::PlatformError;

/// Event-sourced PaymentIntent repository.
/// Primary persistence via event store; read model via payment_intent table.
pub struct PostgresPaymentIntentRepository {
    db: DatabaseConnection,
}

impl PostgresPaymentIntentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentIntentRepository for PostgresPaymentIntentRepository {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentIntent>, PlatformError> {
        // Try event-sourced load first (replay from events)
        let events = self.load_events_from_store(id).await?;
        if !events.is_empty() {
            return Ok(Some(PaymentIntent::from_events(id, events)));
        }

        // Fall back to CRUD read model (for pre-event-sourcing data)
        let model = payment_intent::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, intent: &PaymentIntent) -> Result<(), PlatformError> {
        // 1. Append uncommitted events to event store
        let mut events = intent.clone();
        let uncommitted = events.take_uncommitted_events();
        if !uncommitted.is_empty() {
            let mut current_seq = self.current_sequence(intent.payment_intent_id).await?;
            for event in uncommitted {
                current_seq += 1;
                self.append_event_to_store(
                    intent.payment_intent_id,
                    intent.operator_id,
                    current_seq,
                    &event,
                ).await?;
            }
        }

        // 2. Update read model (payment_intent table) for query performance
        let active_model: payment_intent::ActiveModel = intent.clone().into();
        let existing = payment_intent::Entity::find_by_id(intent.payment_intent_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<PaymentIntent>, PlatformError> {
        let model = payment_intent::Entity::find()
            .filter(payment_intent::Column::IdempotencyKey.eq(key))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save_attempt(&self, attempt: &RoutingAttempt) -> Result<(), PlatformError> {
        let active_model: routing_attempt::ActiveModel = attempt.clone().into();

        let existing = routing_attempt::Entity::find_by_id(attempt.attempt_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn load_attempts(&self, payment_intent_id: Uuid) -> Result<Vec<RoutingAttempt>, PlatformError> {
        let models = routing_attempt::Entity::find()
            .filter(routing_attempt::Column::PaymentIntentId.eq(payment_intent_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }
}

impl PostgresPaymentIntentRepository {
    /// Load events from event store for a payment intent.
    async fn load_events_from_store(&self, aggregate_id: Uuid) -> Result<Vec<PaymentIntentEvent>, PlatformError> {
        let rows = self.db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT payload, event_type FROM event_store WHERE aggregate_type = 'PaymentIntent' AND aggregate_id = $1 ORDER BY event_sequence ASC",
                vec![aggregate_id.into()],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Event store load failed: {e}")))?;

        let mut events = Vec::new();
        for row in rows {
            let payload_json: serde_json::Value = row.try_get("", "payload").unwrap_or_default();
            let event_type: String = row.try_get("", "event_type").unwrap_or_default();

            let event = match event_type.as_str() {
                "PaymentIntentCreated" => PaymentIntentEvent::Created {
                    operator_id: Uuid::parse_str(payload_json["operator_id"].as_str().unwrap_or("")).unwrap_or_default(),
                    amount_minor_units: payload_json["amount_minor_units"].as_i64().unwrap_or(0),
                    currency: payload_json["currency"].as_str().unwrap_or("AED").to_string(),
                    idempotency_key: payload_json["idempotency_key"].as_str().unwrap_or("").to_string(),
                    purpose: payload_json["purpose"].as_str().unwrap_or("payment").to_string(),
                },
                "PaymentAuthorized" => PaymentIntentEvent::Authorized {
                    amount_minor_units: payload_json["amount_minor_units"].as_i64().unwrap_or(0),
                    acquirer_reference: payload_json["acquirer_reference"].as_str().unwrap_or("").to_string(),
                },
                "PaymentCaptured" => PaymentIntentEvent::Captured {
                    captured_amount_minor_units: payload_json["captured_amount_minor_units"].as_i64().unwrap_or(0),
                },
                "PaymentPartiallyCaptured" => PaymentIntentEvent::PartiallyCaptured {
                    captured_amount_minor_units: payload_json["captured_amount_minor_units"].as_i64().unwrap_or(0),
                },
                "PaymentVoided" => PaymentIntentEvent::Voided {},
                "PaymentFailed" => PaymentIntentEvent::Failed {
                    reason: payload_json["reason"].as_str().unwrap_or("").to_string(),
                },
                "PaymentRefunded" => PaymentIntentEvent::Refunded {
                    refund_amount_minor_units: payload_json["refund_amount_minor_units"].as_i64().unwrap_or(0),
                },
                "PaymentPartiallyRefunded" => PaymentIntentEvent::PartiallyRefunded {
                    refund_amount_minor_units: payload_json["refund_amount_minor_units"].as_i64().unwrap_or(0),
                },
                _ => continue,
            };
            events.push(event);
        }

        Ok(events)
    }

    /// Get current sequence number for an aggregate.
    async fn current_sequence(&self, aggregate_id: Uuid) -> Result<i64, PlatformError> {
        let result = self.db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COALESCE(MAX(event_sequence), 0) as seq FROM event_store WHERE aggregate_type = 'PaymentIntent' AND aggregate_id = $1",
                vec![aggregate_id.into()],
            ))
            .await
            .map_err(|e| PlatformError::Internal(format!("Event store sequence query failed: {e}")))?;

        match result {
            Some(row) => Ok(row.try_get("", "seq").unwrap_or(0)),
            None => Ok(0),
        }
    }

    /// Append a single event to the event store.
    async fn append_event_to_store(
        &self,
        aggregate_id: Uuid,
        operator_id: Uuid,
        sequence: i64,
        event: &PaymentIntentEvent,
    ) -> Result<(), PlatformError> {
        let event_id = Uuid::now_v7();
        let (event_type, payload) = match event {
            PaymentIntentEvent::Created { operator_id: oid, amount_minor_units, currency, idempotency_key, purpose } => {
                ("PaymentIntentCreated", serde_json::json!({
                    "operator_id": oid, "amount_minor_units": amount_minor_units,
                    "currency": currency, "idempotency_key": idempotency_key, "purpose": purpose
                }))
            }
            PaymentIntentEvent::Authorized { amount_minor_units, acquirer_reference } => {
                ("PaymentAuthorized", serde_json::json!({
                    "amount_minor_units": amount_minor_units, "acquirer_reference": acquirer_reference
                }))
            }
            PaymentIntentEvent::Captured { captured_amount_minor_units } => {
                ("PaymentCaptured", serde_json::json!({ "captured_amount_minor_units": captured_amount_minor_units }))
            }
            PaymentIntentEvent::PartiallyCaptured { captured_amount_minor_units } => {
                ("PaymentPartiallyCaptured", serde_json::json!({ "captured_amount_minor_units": captured_amount_minor_units }))
            }
            PaymentIntentEvent::Voided {} => ("PaymentVoided", serde_json::json!({})),
            PaymentIntentEvent::Failed { reason } => {
                ("PaymentFailed", serde_json::json!({ "reason": reason }))
            }
            PaymentIntentEvent::Refunded { refund_amount_minor_units } => {
                ("PaymentRefunded", serde_json::json!({ "refund_amount_minor_units": refund_amount_minor_units }))
            }
            PaymentIntentEvent::PartiallyRefunded { refund_amount_minor_units } => {
                ("PaymentPartiallyRefunded", serde_json::json!({ "refund_amount_minor_units": refund_amount_minor_units }))
            }
            PaymentIntentEvent::CaptureStarted { amount_minor_units } => {
                ("PaymentCaptureStarted", serde_json::json!({ "amount_minor_units": amount_minor_units }))
            }
            PaymentIntentEvent::RefundStarted { amount_minor_units } => {
                ("PaymentRefundStarted", serde_json::json!({ "amount_minor_units": amount_minor_units }))
            }
        };

        self.db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO event_store (event_id, aggregate_type, aggregate_id, event_sequence, event_type, event_version, payload, occurred_at, actor_type, actor_id, correlation_id)
             VALUES ($1, 'PaymentIntent', $2, $3, $4, 1, $5, NOW(), 'system', $6, $7)",
            vec![
                event_id.into(),
                aggregate_id.into(),
                sequence.into(),
                event_type.to_string().into(),
                payload.into(),
                operator_id.into(),
                Uuid::now_v7().into(),
            ],
        ))
        .await
        .map_err(|e| PlatformError::Internal(format!("Event store append failed: {e}")))?;

        Ok(())
    }
}

pub struct PostgresRoutingPolicyRepository {
    db: DatabaseConnection,
}

impl PostgresRoutingPolicyRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl crate::infrastructure::repository::RoutingPolicyRepository for PostgresRoutingPolicyRepository {
    async fn load_active_for_operator(&self, operator_id: Uuid) -> Result<Option<RoutingPolicy>, PlatformError> {
        let model = routing_policy::Entity::find()
            .filter(routing_policy::Column::OperatorId.eq(operator_id))
            .filter(routing_policy::Column::Status.eq("active"))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, policy: &RoutingPolicy) -> Result<(), PlatformError> {
        let active_model: routing_policy::ActiveModel = policy.clone().into();

        let existing = routing_policy::Entity::find_by_id(policy.routing_policy_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }
}
