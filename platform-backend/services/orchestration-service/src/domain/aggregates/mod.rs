use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{FailoverConfig, PaymentPurpose, RoutingRule};
use shared_types::{Money, PaymentStatus};

#[derive(Debug, Clone)]
pub struct PaymentIntent {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub status: PaymentStatus,
    pub requested_amount: Money,
    pub authorized_amount: Money,
    pub captured_amount: Money,
    pub refunded_amount: Money,
    pub idempotency_key: String,
    pub payment_method_token_id: Option<Uuid>,
    pub routing_policy_id: Option<Uuid>,
    pub purpose: PaymentPurpose,
    pub metadata: Option<serde_json::Value>,
    pub gateway_profile_id: Option<Uuid>,
    pub gateway_profile_version: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentIntent {
    pub fn new(
        operator_id: Uuid,
        amount: Money,
        idempotency_key: String,
        purpose: PaymentPurpose,
    ) -> Self {
        let now = Utc::now();
        Self {
            payment_intent_id: Uuid::now_v7(),
            operator_id,
            status: PaymentStatus::Created,
            requested_amount: amount.clone(),
            authorized_amount: Money {
                amount_minor_units: 0,
                currency: amount.currency.clone(),
            },
            captured_amount: Money {
                amount_minor_units: 0,
                currency: amount.currency.clone(),
            },
            refunded_amount: Money {
                amount_minor_units: 0,
                currency: amount.currency.clone(),
            },
            idempotency_key,
            payment_method_token_id: None,
            routing_policy_id: None,
            purpose,
            metadata: None,
            gateway_profile_id: None,
            gateway_profile_version: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn can_authorize(&self) -> bool {
        matches!(self.status, PaymentStatus::Created | PaymentStatus::Failed)
    }

    pub fn can_capture(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Authorized | PaymentStatus::PartiallyCaptured
        )
    }

    pub fn can_void(&self) -> bool {
        self.status == PaymentStatus::Authorized
    }

    pub fn can_refund(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Captured | PaymentStatus::PartiallyCaptured
        )
    }

    pub fn remaining_capture_amount(&self) -> i64 {
        self.authorized_amount.amount_minor_units - self.captured_amount.amount_minor_units
    }

    pub fn remaining_refund_amount(&self) -> i64 {
        self.captured_amount.amount_minor_units - self.refunded_amount.amount_minor_units
    }

    pub fn record_authorization(&mut self, amount: Money) {
        self.authorized_amount = amount;
        self.status = PaymentStatus::Authorized;
        self.updated_at = Utc::now();
    }

    pub fn record_capture(&mut self, amount: Money) {
        self.captured_amount = amount;
        if self.captured_amount.amount_minor_units >= self.authorized_amount.amount_minor_units {
            self.status = PaymentStatus::Captured;
        } else {
            self.status = PaymentStatus::PartiallyCaptured;
        }
        self.updated_at = Utc::now();
    }

    pub fn record_refund(&mut self, amount: Money) {
        self.refunded_amount = shared_types::Money {
            amount_minor_units: self.refunded_amount.amount_minor_units + amount.amount_minor_units,
            currency: amount.currency.clone(),
        };
        if self.refunded_amount.amount_minor_units >= self.captured_amount.amount_minor_units {
            self.status = PaymentStatus::Refunded;
        } else {
            self.status = PaymentStatus::PartiallyRefunded;
        }
        self.updated_at = Utc::now();
    }

    pub fn record_void(&mut self) {
        self.status = PaymentStatus::Voided;
        self.updated_at = Utc::now();
    }

    pub fn record_failure(&mut self) {
        self.status = PaymentStatus::Failed;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub status: String,
    pub rules: Vec<RoutingRule>,
    pub failover_config: FailoverConfig,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

impl RoutingPolicy {
    pub fn new(operator_id: Uuid) -> Self {
        Self {
            routing_policy_id: Uuid::now_v7(),
            operator_id,
            version: 1,
            status: "active".to_string(),
            rules: Vec::new(),
            failover_config: FailoverConfig::default(),
            created_at: Utc::now(),
            activated_at: None,
        }
    }

    pub fn select_route(
        &self,
        card_scheme: Option<&shared_types::CardScheme>,
        currency: &shared_types::CurrencyCode,
        amount: &Money,
        attempted_links: &[Uuid],
    ) -> Option<Uuid> {
        let mut candidates: Vec<(&RoutingRule, i32)> = self
            .rules
            .iter()
            .filter(|r| r.matches(card_scheme, currency, amount))
            .filter(|r| !attempted_links.contains(&r.acquirer_link_id))
            .map(|r| (r, r.priority))
            .collect();

        candidates.sort_by_key(|(_, priority)| *priority);

        candidates.first().map(|(rule, _)| rule.acquirer_link_id)
    }
}

#[derive(Debug, Clone)]
pub struct RoutingAttempt {
    pub attempt_id: Uuid,
    pub payment_intent_id: Uuid,
    pub attempt_number: i32,
    pub acquirer_link_id: Uuid,
    pub connector_id: String,
    pub status: String,
    pub decline_reason: Option<String>,
    pub acquirer_reference: Option<String>,
    pub latency_ms: u32,
    pub attempted_at: DateTime<Utc>,
}

impl RoutingAttempt {
    pub fn new(payment_intent_id: Uuid, attempt_number: i32, acquirer_link_id: Uuid, connector_id: String) -> Self {
        Self {
            attempt_id: Uuid::now_v7(),
            payment_intent_id,
            attempt_number,
            acquirer_link_id,
            connector_id,
            status: "pending".to_string(),
            decline_reason: None,
            acquirer_reference: None,
            latency_ms: 0,
            attempted_at: Utc::now(),
        }
    }
}
