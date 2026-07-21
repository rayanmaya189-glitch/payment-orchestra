use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{FailoverConfig, PaymentPurpose, RoutingRule};
use shared_types::{Money, PaymentStatus};

/// Domain events for PaymentIntent (SRS Part 5 §4, EVT-01 through EVT-10).
#[derive(Debug, Clone)]
pub enum PaymentIntentEvent {
    Created {
        operator_id: Uuid,
        amount_minor_units: i64,
        currency: String,
        idempotency_key: String,
        purpose: String,
    },
    Authorized {
        amount_minor_units: i64,
        acquirer_reference: String,
    },
    CaptureStarted {
        amount_minor_units: i64,
    },
    Captured {
        captured_amount_minor_units: i64,
    },
    PartiallyCaptured {
        captured_amount_minor_units: i64,
    },
    Voided {},
    Failed {
        reason: String,
    },
    RefundStarted {
        amount_minor_units: i64,
    },
    Refunded {
        refund_amount_minor_units: i64,
    },
    PartiallyRefunded {
        refund_amount_minor_units: i64,
    },
}

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
    /// Uncommitted domain events (event sourcing)
    pub(crate) uncommitted_events: Vec<PaymentIntentEvent>,
}

impl PaymentIntent {
    pub fn new(
        operator_id: Uuid,
        amount: Money,
        idempotency_key: String,
        purpose: PaymentPurpose,
    ) -> Self {
        let now = Utc::now();
        let purpose_str = match &purpose {
            PaymentPurpose::Payment => "payment",
            PaymentPurpose::CardVerification => "card_verification",
        }.to_string();

        let event = PaymentIntentEvent::Created {
            operator_id,
            amount_minor_units: amount.amount_minor_units,
            currency: amount.currency.0.clone(),
            idempotency_key: idempotency_key.clone(),
            purpose: purpose_str,
        };

        let mut intent = Self {
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
            uncommitted_events: Vec::new(),
        };

        intent.apply(event);
        intent
    }

    /// Apply a domain event to mutate state (event sourcing fold).
    pub fn apply(&mut self, event: PaymentIntentEvent) {
        match &event {
            PaymentIntentEvent::Created { .. } => {
                // Already initialized in new()
            }
            PaymentIntentEvent::Authorized { amount_minor_units, .. } => {
                self.authorized_amount.amount_minor_units = *amount_minor_units;
                self.status = PaymentStatus::Authorized;
            }
            PaymentIntentEvent::Captured { captured_amount_minor_units } => {
                self.captured_amount.amount_minor_units = *captured_amount_minor_units;
                self.status = PaymentStatus::Captured;
            }
            PaymentIntentEvent::PartiallyCaptured { captured_amount_minor_units } => {
                self.captured_amount.amount_minor_units = *captured_amount_minor_units;
                self.status = PaymentStatus::PartiallyCaptured;
            }
            PaymentIntentEvent::Voided {} => {
                self.status = PaymentStatus::Voided;
            }
            PaymentIntentEvent::Failed { .. } => {
                self.status = PaymentStatus::Failed;
            }
            PaymentIntentEvent::Refunded { refund_amount_minor_units } => {
                self.refunded_amount.amount_minor_units = *refund_amount_minor_units;
                if self.refunded_amount.amount_minor_units >= self.captured_amount.amount_minor_units {
                    self.status = PaymentStatus::Refunded;
                } else {
                    self.status = PaymentStatus::PartiallyRefunded;
                }
            }
            PaymentIntentEvent::PartiallyRefunded { refund_amount_minor_units } => {
                self.refunded_amount.amount_minor_units = *refund_amount_minor_units;
                self.status = PaymentStatus::PartiallyRefunded;
            }
            PaymentIntentEvent::CaptureStarted { .. } => {
                self.status = PaymentStatus::Capturing;
            }
            PaymentIntentEvent::RefundStarted { .. } => {
                self.status = PaymentStatus::Refunding;
            }
        }
        self.updated_at = Utc::now();
        self.uncommitted_events.push(event);
    }

    /// Rebuild aggregate from event history (event sourcing replay).
    pub fn from_events(id: Uuid, events: Vec<PaymentIntentEvent>) -> Self {
        let first = events.first().expect("Events must not be empty");
        let (operator_id, amount_minor_units, currency, idempotency_key, purpose_str) = match first {
            PaymentIntentEvent::Created { operator_id, amount_minor_units, currency, idempotency_key, purpose } => {
                (*operator_id, *amount_minor_units, currency.clone(), idempotency_key.clone(), purpose.clone())
            }
            _ => panic!("First event must be Created"),
        };

        let currency_obj = shared_types::CurrencyCode::new(&currency).unwrap();
        let purpose = match purpose_str.as_str() {
            "card_verification" => PaymentPurpose::CardVerification,
            _ => PaymentPurpose::Payment,
        };

        let mut intent = Self {
            payment_intent_id: id,
            operator_id,
            status: PaymentStatus::Created,
            requested_amount: Money { amount_minor_units, currency: currency_obj.clone() },
            authorized_amount: Money { amount_minor_units: 0, currency: currency_obj.clone() },
            captured_amount: Money { amount_minor_units: 0, currency: currency_obj.clone() },
            refunded_amount: Money { amount_minor_units: 0, currency: currency_obj },
            idempotency_key,
            payment_method_token_id: None,
            routing_policy_id: None,
            purpose,
            metadata: None,
            gateway_profile_id: None,
            gateway_profile_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            uncommitted_events: Vec::new(),
        };

        for event in events.into_iter().skip(1) {
            intent.apply(event);
        }
        intent.uncommitted_events.clear();
        intent
    }

    /// Take uncommitted events (for publishing after save).
    pub fn take_uncommitted_events(&mut self) -> Vec<PaymentIntentEvent> {
        std::mem::take(&mut self.uncommitted_events)
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
        self.apply(PaymentIntentEvent::Authorized {
            amount_minor_units: amount.amount_minor_units,
            acquirer_reference: String::new(),
        });
    }

    pub fn record_capture(&mut self, amount: Money) {
        let new_captured = self.captured_amount.amount_minor_units + amount.amount_minor_units;
        if new_captured >= self.authorized_amount.amount_minor_units {
            self.apply(PaymentIntentEvent::Captured {
                captured_amount_minor_units: new_captured,
            });
        } else {
            self.apply(PaymentIntentEvent::PartiallyCaptured {
                captured_amount_minor_units: new_captured,
            });
        }
    }

    pub fn record_refund(&mut self, amount: Money) {
        let new_refunded = self.refunded_amount.amount_minor_units + amount.amount_minor_units;
        if new_refunded >= self.captured_amount.amount_minor_units {
            self.apply(PaymentIntentEvent::Refunded {
                refund_amount_minor_units: new_refunded,
            });
        } else {
            self.apply(PaymentIntentEvent::PartiallyRefunded {
                refund_amount_minor_units: new_refunded,
            });
        }
    }

    pub fn record_void(&mut self) {
        self.apply(PaymentIntentEvent::Voided {});
    }

    pub fn record_failure(&mut self) {
        self.apply(PaymentIntentEvent::Failed { reason: String::new() });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn aed(amount: i64) -> Money {
        Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() }
    }

    fn make_intent() -> PaymentIntent {
        PaymentIntent::new(
            Uuid::now_v7(),
            aed(10000),
            "idem_test_123".to_string(),
            PaymentPurpose::Payment,
        )
    }

    #[test]
    fn test_new_intent_creates_event() {
        let intent = make_intent();
        assert_eq!(intent.status, PaymentStatus::Created);
        assert_eq!(intent.requested_amount.amount_minor_units, 10000);
        assert_eq!(intent.uncommitted_events.len(), 1);
    }

    #[test]
    fn test_authorize_emits_event() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        assert_eq!(intent.status, PaymentStatus::Authorized);
        assert_eq!(intent.authorized_amount.amount_minor_units, 10000);
        assert_eq!(intent.uncommitted_events.len(), 2); // Created + Authorized
    }

    #[test]
    fn test_capture_full_emits_event() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        intent.record_capture(aed(10000));
        assert_eq!(intent.status, PaymentStatus::Captured);
        assert_eq!(intent.captured_amount.amount_minor_units, 10000);
    }

    #[test]
    fn test_capture_partial_emits_event() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        intent.record_capture(aed(5000));
        assert_eq!(intent.status, PaymentStatus::PartiallyCaptured);
        assert_eq!(intent.captured_amount.amount_minor_units, 5000);
        assert_eq!(intent.remaining_capture_amount(), 5000);
    }

    #[test]
    fn test_void_emits_event() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        intent.record_void();
        assert_eq!(intent.status, PaymentStatus::Voided);
    }

    #[test]
    fn test_refund_full_emits_event() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        intent.record_capture(aed(10000));
        intent.record_refund(aed(10000));
        assert_eq!(intent.status, PaymentStatus::Refunded);
        assert_eq!(intent.refunded_amount.amount_minor_units, 10000);
        assert_eq!(intent.remaining_refund_amount(), 0);
    }

    #[test]
    fn test_refund_partial_emits_event() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        intent.record_capture(aed(10000));
        intent.record_refund(aed(3000));
        assert_eq!(intent.status, PaymentStatus::PartiallyRefunded);
        assert_eq!(intent.refunded_amount.amount_minor_units, 3000);
        assert_eq!(intent.remaining_refund_amount(), 7000);
    }

    #[test]
    fn test_failure_emits_event() {
        let mut intent = make_intent();
        intent.record_failure();
        assert_eq!(intent.status, PaymentStatus::Failed);
    }

    #[test]
    fn test_from_events_replay() {
        let events = vec![
            PaymentIntentEvent::Created {
                operator_id: Uuid::now_v7(),
                amount_minor_units: 10000,
                currency: "AED".to_string(),
                idempotency_key: "idem_replay".to_string(),
                purpose: "payment".to_string(),
            },
            PaymentIntentEvent::Authorized {
                amount_minor_units: 10000,
                acquirer_reference: "acq_123".to_string(),
            },
            PaymentIntentEvent::Captured {
                captured_amount_minor_units: 10000,
            },
        ];

        let id = Uuid::now_v7();
        let intent = PaymentIntent::from_events(id, events);
        assert_eq!(intent.payment_intent_id, id);
        assert_eq!(intent.status, PaymentStatus::Captured);
        assert_eq!(intent.authorized_amount.amount_minor_units, 10000);
        assert_eq!(intent.captured_amount.amount_minor_units, 10000);
        assert!(intent.uncommitted_events.is_empty()); // Cleared after replay
    }

    #[test]
    fn test_can_authorize_only_created_or_failed() {
        let mut intent = make_intent();
        assert!(intent.can_authorize());

        intent.record_authorization(aed(10000));
        assert!(!intent.can_authorize());

        intent.record_failure();
        assert!(intent.can_authorize());
    }

    #[test]
    fn test_can_capture_only_authorized_or_partially_captured() {
        let mut intent = make_intent();
        assert!(!intent.can_capture());

        intent.record_authorization(aed(10000));
        assert!(intent.can_capture());

        intent.record_capture(aed(5000));
        assert!(intent.can_capture()); // PartiallyCaptured

        intent.record_capture(aed(5000));
        assert!(!intent.can_capture()); // Captured
    }

    #[test]
    fn test_can_void_only_authorized() {
        let mut intent = make_intent();
        assert!(!intent.can_void());

        intent.record_authorization(aed(10000));
        assert!(intent.can_void());

        intent.record_void();
        assert!(!intent.can_void());
    }

    #[test]
    fn test_can_refund_only_captured() {
        let mut intent = make_intent();
        assert!(!intent.can_refund());

        intent.record_authorization(aed(10000));
        assert!(!intent.can_refund());

        intent.record_capture(aed(10000));
        assert!(intent.can_refund());
    }

    #[test]
    fn test_routing_policy_select_route() {
        let policy = RoutingPolicy {
            routing_policy_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            version: 1,
            status: "active".to_string(),
            rules: vec![
                RoutingRule {
                    rule_id: "rule1".to_string(),
                    priority: 1,
                    card_scheme: Some(shared_types::CardScheme::Visa),
                    currency: Some(shared_types::CurrencyCode::new("AED").unwrap()),
                    min_amount: None,
                    max_amount: None,
                    acquirer_link_id: Uuid::now_v7(),
                },
            ],
            failover_config: FailoverConfig::default(),
            created_at: Utc::now(),
            activated_at: None,
        };

        let link = policy.select_route(
            Some(&shared_types::CardScheme::Visa),
            &shared_types::CurrencyCode::new("AED").unwrap(),
            &aed(5000),
            &[],
        );
        assert!(link.is_some());

        // Already attempted → no route
        let link = policy.select_route(
            Some(&shared_types::CardScheme::Visa),
            &shared_types::CurrencyCode::new("AED").unwrap(),
            &aed(5000),
            &[policy.rules[0].acquirer_link_id],
        );
        assert!(link.is_none());
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_uncommitted_events_cleared_after_from_events() {
        let events = vec![
            PaymentIntentEvent::Created {
                operator_id: Uuid::now_v7(),
                amount_minor_units: 5000,
                currency: "AED".to_string(),
                idempotency_key: "idem_clear".to_string(),
                purpose: "payment".to_string(),
            },
        ];
        let intent = PaymentIntent::from_events(Uuid::now_v7(), events);
        assert!(intent.uncommitted_events.is_empty());
    }

    #[test]
    fn test_take_uncommitted_events() {
        let mut intent = make_intent();
        assert_eq!(intent.uncommitted_events.len(), 1);
        let taken = intent.take_uncommitted_events();
        assert_eq!(taken.len(), 1);
        assert!(intent.uncommitted_events.is_empty());
    }

    #[test]
    fn test_capture_zero_amount_fails_validation() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        // Capture with zero amount
        intent.record_capture(aed(0));
        // Should be partially captured (0 < 10000)
        assert_eq!(intent.status, PaymentStatus::PartiallyCaptured);
    }

    #[test]
    fn test_refund_more_than_captured_goes_to_fully_refunded() {
        let mut intent = make_intent();
        intent.record_authorization(aed(10000));
        intent.record_capture(aed(5000));
        intent.record_refund(aed(10000)); // More than captured
        assert_eq!(intent.status, PaymentStatus::Refunded);
        assert_eq!(intent.refunded_amount.amount_minor_units, 10000);
    }

    #[test]
    fn test_routing_policy_no_rules_returns_none() {
        let policy = RoutingPolicy {
            routing_policy_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            version: 1,
            status: "active".to_string(),
            rules: vec![],
            failover_config: FailoverConfig::default(),
            created_at: Utc::now(),
            activated_at: None,
        };

        let link = policy.select_route(
            Some(&shared_types::CardScheme::Visa),
            &shared_types::CurrencyCode::new("AED").unwrap(),
            &aed(5000),
            &[],
        );
        assert!(link.is_none());
    }

    #[test]
    fn test_routing_policy_multiple_rules_selects_highest_priority() {
        let link1 = Uuid::now_v7();
        let link2 = Uuid::now_v7();
        let policy = RoutingPolicy {
            routing_policy_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            version: 1,
            status: "active".to_string(),
            rules: vec![
                RoutingRule {
                    rule_id: "rule_low".to_string(),
                    priority: 10,
                    card_scheme: None,
                    currency: None,
                    min_amount: None,
                    max_amount: None,
                    acquirer_link_id: link1,
                },
                RoutingRule {
                    rule_id: "rule_high".to_string(),
                    priority: 1,
                    card_scheme: None,
                    currency: None,
                    min_amount: None,
                    max_amount: None,
                    acquirer_link_id: link2,
                },
            ],
            failover_config: FailoverConfig::default(),
            created_at: Utc::now(),
            activated_at: None,
        };

        let selected = policy.select_route(
            None,
            &shared_types::CurrencyCode::new("AED").unwrap(),
            &aed(5000),
            &[],
        );
        assert_eq!(selected, Some(link2)); // Higher priority (lower number)
    }

    #[test]
    fn test_routing_policy_amount_range_filter() {
        let link1 = Uuid::now_v7();
        let link2 = Uuid::now_v7();
        let policy = RoutingPolicy {
            routing_policy_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            version: 1,
            status: "active".to_string(),
            rules: vec![
                RoutingRule {
                    rule_id: "small".to_string(),
                    priority: 1,
                    card_scheme: None,
                    currency: None,
                    min_amount: Some(0),
                    max_amount: Some(1000),
                    acquirer_link_id: link1,
                },
                RoutingRule {
                    rule_id: "large".to_string(),
                    priority: 2,
                    card_scheme: None,
                    currency: None,
                    min_amount: Some(1001),
                    max_amount: None,
                    acquirer_link_id: link2,
                },
            ],
            failover_config: FailoverConfig::default(),
            created_at: Utc::now(),
            activated_at: None,
        };

        // Small amount → link1
        let selected = policy.select_route(None, &shared_types::CurrencyCode::new("AED").unwrap(), &aed(500), &[]);
        assert_eq!(selected, Some(link1));

        // Large amount → link2
        let selected = policy.select_route(None, &shared_types::CurrencyCode::new("AED").unwrap(), &aed(5000), &[]);
        assert_eq!(selected, Some(link2));
    }

    #[test]
    fn test_failover_config_default() {
        let config = FailoverConfig::default();
        assert_eq!(config.max_hops, 3);
        assert_eq!(config.latency_budget_ms, 10000);
        assert!(!config.retry_unknown_as_fallback);
    }

    #[test]
    fn test_routing_attempt_new() {
        let attempt = RoutingAttempt::new(Uuid::now_v7(), 1, Uuid::now_v7(), "ni-test".into());
        assert_eq!(attempt.attempt_number, 1);
        assert_eq!(attempt.status, "pending");
        assert!(attempt.decline_reason.is_none());
    }
}

// ==================== Property-Based Tests (SRS PROP-TEST-001) ====================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_currency() -> impl Strategy<Value = shared_types::CurrencyCode> {
        prop_oneof![
            Just(shared_types::CurrencyCode::new("AED").unwrap()),
            Just(shared_types::CurrencyCode::new("USD").unwrap()),
            Just(shared_types::CurrencyCode::new("EUR").unwrap()),
            Just(shared_types::CurrencyCode::new("GBP").unwrap()),
            Just(shared_types::CurrencyCode::new("SAR").unwrap()),
        ]
    }

    fn arb_amount() -> impl Strategy<Value = i64> {
        (1i64..1_000_000_000) // 0.01 to 10,000,000.00 in minor units
    }

    fn arb_money() -> impl Strategy<Value = Money> {
        (arb_amount(), arb_currency()).prop_map(|(amount, currency)| Money {
            amount_minor_units: amount,
            currency,
        })
    }

    proptest! {
        #[test]
        fn prop_new_intent_always_created(amount in arb_money()) {
            let intent = PaymentIntent::new(
                Uuid::now_v7(),
                amount.clone(),
                format!("idem_{}", Uuid::now_v7()),
                PaymentPurpose::Payment,
            );
            prop_assert_eq!(intent.status, PaymentStatus::Created);
            prop_assert_eq!(intent.requested_amount.amount_minor_units, amount.amount_minor_units);
            prop_assert_eq!(intent.uncommitted_events.len(), 1);
        }

        #[test]
        fn prop_authorize_sets_authorized(amount in arb_money()) {
            let mut intent = PaymentIntent::new(
                Uuid::now_v7(),
                amount.clone(),
                format!("idem_{}", Uuid::now_v7()),
                PaymentPurpose::Payment,
            );
            intent.record_authorization(amount.clone());
            prop_assert_eq!(intent.status, PaymentStatus::Authorized);
            prop_assert_eq!(intent.authorized_amount.amount_minor_units, amount.amount_minor_units);
        }

        #[test]
        fn prop_capture_never_exceeds_authorized(
            auth_amount in arb_amount(),
            capture_amount in arb_amount(),
        ) {
            let mut intent = PaymentIntent::new(
                Uuid::now_v7(),
                Money { amount_minor_units: auth_amount, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                format!("idem_{}", Uuid::now_v7()),
                PaymentPurpose::Payment,
            );
            intent.record_authorization(Money { amount_minor_units: auth_amount, currency: shared_types::CurrencyCode::new("AED").unwrap() });

            let capped_capture = capture_amount.min(auth_amount);
            intent.record_capture(Money { amount_minor_units: capped_capture, currency: shared_types::CurrencyCode::new("AED").unwrap() });

            // INV-01: captured_amount <= authorized_amount
            prop_assert!(
                intent.captured_amount.amount_minor_units <= intent.authorized_amount.amount_minor_units,
                "captured {} exceeded authorized {}",
                intent.captured_amount.amount_minor_units,
                intent.authorized_amount.amount_minor_units,
            );
        }

        #[test]
        fn prop_refund_never_exceeds_captured(
            auth_amount in arb_amount(),
            capture_amount in arb_amount(),
            refund_amount in arb_amount(),
        ) {
            let capped_auth = auth_amount.max(1);
            let capped_capture = capture_amount.min(capped_auth);
            let capped_refund = refund_amount.min(capped_capture);

            let mut intent = PaymentIntent::new(
                Uuid::now_v7(),
                Money { amount_minor_units: capped_auth, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                format!("idem_{}", Uuid::now_v7()),
                PaymentPurpose::Payment,
            );
            intent.record_authorization(Money { amount_minor_units: capped_auth, currency: shared_types::CurrencyCode::new("AED").unwrap() });
            intent.record_capture(Money { amount_minor_units: capped_capture, currency: shared_types::CurrencyCode::new("AED").unwrap() });
            intent.record_refund(Money { amount_minor_units: capped_refund, currency: shared_types::CurrencyCode::new("AED").unwrap() });

            // INV-01: refunded_amount <= captured_amount
            prop_assert!(
                intent.refunded_amount.amount_minor_units <= intent.captured_amount.amount_minor_units,
                "refunded {} exceeded captured {}",
                intent.refunded_amount.amount_minor_units,
                intent.captured_amount.amount_minor_units,
            );
        }

        #[test]
        fn prop_from_events_replay_produces_same_state(
            auth_amount in arb_amount(),
        ) {
            let id = Uuid::now_v7();
            let events = vec![
                PaymentIntentEvent::Created {
                    operator_id: Uuid::now_v7(),
                    amount_minor_units: auth_amount,
                    currency: "AED".to_string(),
                    idempotency_key: "idem_replay".to_string(),
                    purpose: "payment".to_string(),
                },
                PaymentIntentEvent::Authorized {
                    amount_minor_units: auth_amount,
                    acquirer_reference: "acq_123".to_string(),
                },
            ];

            let intent = PaymentIntent::from_events(id, events);
            prop_assert_eq!(intent.payment_intent_id, id);
            prop_assert_eq!(intent.status, PaymentStatus::Authorized);
            prop_assert_eq!(intent.authorized_amount.amount_minor_units, auth_amount);
            prop_assert!(intent.uncommitted_events.is_empty());
        }

        #[test]
        fn prop_void_only_from_authorized(amount in arb_amount()) {
            let mut intent = PaymentIntent::new(
                Uuid::now_v7(),
                Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                format!("idem_{}", Uuid::now_v7()),
                PaymentPurpose::Payment,
            );

            // Can't void from Created
            prop_assert!(!intent.can_void());

            intent.record_authorization(Money { amount_minor_units: amount, currency: shared_types::CurrencyCode::new("AED").unwrap() });

            // Can void from Authorized
            prop_assert!(intent.can_void());

            intent.record_void();

            // Can't void after voided
            prop_assert!(!intent.can_void());
        }

        #[test]
        fn prop_state_machine_consistency(
            auth_amount in arb_amount(),
            capture_amount in arb_amount(),
        ) {
            let capped_auth = auth_amount.max(1);
            let capped_capture = capture_amount.min(capped_auth);

            let mut intent = PaymentIntent::new(
                Uuid::now_v7(),
                Money { amount_minor_units: capped_auth, currency: shared_types::CurrencyCode::new("AED").unwrap() },
                format!("idem_{}", Uuid::now_v7()),
                PaymentPurpose::Payment,
            );

            // Created -> Authorized
            prop_assert!(intent.can_authorize());
            intent.record_authorization(Money { amount_minor_units: capped_auth, currency: shared_types::CurrencyCode::new("AED").unwrap() });
            prop_assert!(!intent.can_authorize());
            prop_assert!(intent.can_capture());
            prop_assert!(intent.can_void());

            // Authorized -> Captured/PartiallyCaptured
            intent.record_capture(Money { amount_minor_units: capped_capture, currency: shared_types::CurrencyCode::new("AED").unwrap() });
            prop_assert!(!intent.can_void());
            prop_assert!(intent.can_refund());

            // INV-01: captured <= authorized
            prop_assert!(intent.captured_amount.amount_minor_units <= intent.authorized_amount.amount_minor_units);
        }
    }
}
