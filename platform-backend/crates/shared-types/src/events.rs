use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub event_version: u32,
    pub occurred_at: DateTime<Utc>,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub payload: serde_json::Value,
    pub trace_context: Option<String>,
    /// HMAC-SHA256 signature for event integrity verification (SRS AUD-004)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl EventEnvelope {
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: Uuid,
        event_type: impl Into<String>,
        actor_type: impl Into<String>,
        correlation_id: Uuid,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            aggregate_type: aggregate_type.into(),
            aggregate_id,
            event_type: event_type.into(),
            event_version: 1,
            occurred_at: Utc::now(),
            actor_type: actor_type.into(),
            actor_id: None,
            causation_id: None,
            correlation_id,
            payload,
            trace_context: None,
            signature: None,
        }
    }

    /// Validate that the payload is valid JSON and not empty.
    /// Returns Err if the payload is `Null` (missing required event data).
    pub fn validate_payload(&self) -> Result<(), &'static str> {
        if self.payload.is_null() {
            return Err("event payload must not be null");
        }
        Ok(())
    }
}

pub trait DomainEvent {
    fn event_type(&self) -> &'static str;
    fn aggregate_type(&self) -> &'static str;
}

// ==================== Typed Event Payloads (SRS EVT-01 through EVT-19) ====================

/// Typed event payloads for core payment events.
/// These replace untyped serde_json::Value for type safety.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum PaymentEventPayload {
    #[serde(rename = "PaymentIntentCreated")]
    IntentCreated {
        operator_id: Uuid,
        amount_minor_units: i64,
        currency: String,
        idempotency_key: String,
    },
    #[serde(rename = "PaymentAuthorizationAttempted")]
    AuthorizationAttempted {
        attempt_number: i32,
        connector_id: String,
        gateway_profile_id: Uuid,
    },
    #[serde(rename = "PaymentAuthorized")]
    Authorized {
        acquirer_reference: String,
        authorized_amount_minor_units: i64,
    },
    #[serde(rename = "PaymentCaptured")]
    Captured {
        captured_amount_minor_units: i64,
        acquirer_reference: String,
    },
    #[serde(rename = "PaymentFailed")]
    Failed {
        decline_reason: String,
        connector_id: String,
    },
    #[serde(rename = "PaymentVoided")]
    Voided {
        acquirer_reference: String,
    },
    #[serde(rename = "PaymentRefunded")]
    Refunded {
        refund_amount_minor_units: i64,
        acquirer_reference: String,
    },
}

impl PaymentEventPayload {
    /// Convert to EventEnvelope with proper event_type and payload.
    pub fn to_envelope(
        &self,
        aggregate_id: Uuid,
        actor_type: &str,
        correlation_id: Uuid,
    ) -> EventEnvelope {
        let (event_type, payload) = match self {
            Self::IntentCreated { .. } => ("PaymentIntentCreated", serde_json::to_value(self).unwrap()),
            Self::AuthorizationAttempted { .. } => ("PaymentAuthorizationAttempted", serde_json::to_value(self).unwrap()),
            Self::Authorized { .. } => ("PaymentAuthorized", serde_json::to_value(self).unwrap()),
            Self::Captured { .. } => ("PaymentCaptured", serde_json::to_value(self).unwrap()),
            Self::Failed { .. } => ("PaymentFailed", serde_json::to_value(self).unwrap()),
            Self::Voided { .. } => ("PaymentVoided", serde_json::to_value(self).unwrap()),
            Self::Refunded { .. } => ("PaymentRefunded", serde_json::to_value(self).unwrap()),
        };

        EventEnvelope::new("PaymentIntent", aggregate_id, event_type, actor_type, correlation_id, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_envelope_validate_payload_ok() {
        let envelope = EventEnvelope::new(
            "PaymentIntent",
            Uuid::now_v7(),
            "PaymentIntentCreated",
            "user",
            Uuid::now_v7(),
            serde_json::json!({"amount": 1000}),
        );
        assert!(envelope.validate_payload().is_ok());
    }

    #[test]
    fn test_event_envelope_validate_payload_rejects_null() {
        let envelope = EventEnvelope::new(
            "PaymentIntent",
            Uuid::now_v7(),
            "PaymentIntentCreated",
            "user",
            Uuid::now_v7(),
            serde_json::Value::Null,
        );
        assert!(envelope.validate_payload().is_err());
    }

    #[test]
    fn test_payment_event_payload_serialization() {
        let payload = PaymentEventPayload::IntentCreated {
            operator_id: Uuid::now_v7(),
            amount_minor_units: 5000,
            currency: "AED".to_string(),
            idempotency_key: "idem_123".to_string(),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["event"], "PaymentIntentCreated");
        assert_eq!(json["data"]["amount_minor_units"], 5000);
    }

    #[test]
    fn test_payment_event_to_envelope() {
        let payload = PaymentEventPayload::Authorized {
            acquirer_reference: "acq_ref_123".to_string(),
            authorized_amount_minor_units: 10000,
        };

        let aggregate_id = Uuid::now_v7();
        let correlation_id = Uuid::now_v7();
        let envelope = payload.to_envelope(aggregate_id, "user", correlation_id);

        assert_eq!(envelope.aggregate_type, "PaymentIntent");
        assert_eq!(envelope.event_type, "PaymentAuthorized");
        assert_eq!(envelope.aggregate_id, aggregate_id);
        assert_eq!(envelope.correlation_id, correlation_id);
    }

    #[test]
    fn test_event_envelope_new_sets_defaults() {
        let id = Uuid::now_v7();
        let correlation = Uuid::now_v7();
        let payload = serde_json::json!({"test": true});

        let envelope = EventEnvelope::new("TestAggregate", id, "TestEvent", "system", correlation, payload.clone());

        assert_eq!(envelope.event_version, 1);
        assert_eq!(envelope.actor_type, "system");
        assert!(envelope.actor_id.is_none());
        assert!(envelope.causation_id.is_none());
        assert!(envelope.trace_context.is_none());
        assert!(envelope.signature.is_none());
        assert_eq!(envelope.payload, payload);
    }
}
