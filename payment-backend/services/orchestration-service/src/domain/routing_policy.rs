use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::OrchestrationError;

// ─── Routing Rule ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub acquirer_link_id: Uuid,
    pub priority: i32,
    pub condition: RoutingCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub card_schemes: Option<Vec<String>>,
    pub currencies: Option<Vec<String>>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
}

impl RoutingCondition {
    pub fn all() -> Self {
        Self { card_schemes: None, currencies: None, min_amount_minor: None, max_amount_minor: None }
    }

    pub fn matches(&self, card_scheme: &str, currency: &str, amount_minor: i64) -> bool {
        if let Some(schemes) = &self.card_schemes {
            if !schemes.iter().any(|s| s == card_scheme) { return false; }
        }
        if let Some(currencies) = &self.currencies {
            if !currencies.iter().any(|c| c == currency) { return false; }
        }
        if let Some(min) = self.min_amount_minor {
            if amount_minor < min { return false; }
        }
        if let Some(max) = self.max_amount_minor {
            if amount_minor > max { return false; }
        }
        true
    }
}

// ─── Strategy Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    Priority,
    RoundRobin,
    WeightedRoundRobin { weights: Vec<(Uuid, u32)> },
    CostBased,
    SuccessRateBased,
    VolumeCapped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartialAuthStrategy {
    AcceptPartial,
    RetryNextAcquirer,
    Reject,
}

// ─── Failover Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub max_hops: u8,
    pub latency_budget_ms: u32,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self { max_hops: 3, latency_budget_ms: 10000 }
    }
}

// ─── Routing Policy Aggregate ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub routing_policy_id: Uuid,
    pub operator_id: Uuid,
    pub version: i32,
    pub status: PolicyStatus,
    pub rules: Vec<RoutingRule>,
    pub failover_config: FailoverConfig,
    pub partial_auth_strategy: PartialAuthStrategy,
    pub rotation_strategy: RotationStrategy,
    pub max_transaction_amount_minor: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyStatus {
    Active,
    Inactive,
}

impl std::fmt::Display for PolicyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyStatus::Active => write!(f, "active"),
            PolicyStatus::Inactive => write!(f, "inactive"),
        }
    }
}

impl RoutingPolicy {
    pub fn new(routing_policy_id: Uuid, operator_id: Uuid, rules: Vec<RoutingRule>) -> Self {
        Self {
            routing_policy_id,
            operator_id,
            version: 1,
            status: PolicyStatus::Inactive,
            rules,
            failover_config: FailoverConfig::default(),
            partial_auth_strategy: PartialAuthStrategy::AcceptPartial,
            rotation_strategy: RotationStrategy::Priority,
            max_transaction_amount_minor: None,
            created_at: Utc::now(),
            activated_at: None,
        }
    }

    /// Select a gateway profile based on routing rules and conditions.
    pub fn select_route(
        &self,
        card_scheme: &str,
        currency: &str,
        amount_minor: i64,
        attempted_hops: &[Uuid],
        available_links: &[Uuid],
    ) -> Result<Uuid, OrchestrationError> {
        for rule in &self.rules {
            if !rule.condition.matches(card_scheme, currency, amount_minor) {
                continue;
            }
            if attempted_hops.contains(&rule.acquirer_link_id) {
                continue;
            }
            if !available_links.contains(&rule.acquirer_link_id) {
                continue;
            }
            return Ok(rule.acquirer_link_id);
        }
        Err(OrchestrationError::NoEligibleRoute)
    }
}
