//! Cost optimization routing for payment gateways.
//!
//! This module provides:
//! - Transaction fee tracking per gateway
//! - Cost-based routing decisions
//! - Fee comparison and optimization
//! - Budget management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Gateway fee structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayFeeStructure {
    pub gateway_id: Uuid,
    /// Fixed fee per transaction (in minor units)
    pub fixed_fee_minor: i64,
    /// Percentage fee in basis points (e.g., 150 = 1.5%)
    pub percentage_fee_bps: u32,
    /// Cross-border fee in basis points
    pub cross_border_fee_bps: Option<u32>,
    /// Currency conversion fee in basis points
    pub currency_conversion_fee_bps: Option<u32>,
    /// Minimum fee per transaction (in minor units)
    pub min_fee_minor: Option<i64>,
    /// Maximum fee per transaction (in minor units)
    pub max_fee_minor: Option<i64>,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Transaction cost breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCost {
    pub gateway_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub fixed_fee: i64,
    pub percentage_fee: i64,
    pub cross_border_fee: i64,
    pub currency_conversion_fee: i64,
    pub total_fee: i64,
    pub effective_rate_bps: u32,
}

/// Cost optimization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationConfig {
    /// Enable cost-based routing
    pub enabled: bool,
    /// Weight for cost in routing decisions (0.0 - 1.0)
    pub cost_weight: f64,
    /// Maximum acceptable fee rate (in basis points)
    pub max_fee_rate_bps: u32,
    /// Enable cross-border fee optimization
    pub optimize_cross_border: bool,
    /// Enable currency conversion fee optimization
    pub optimize_currency_conversion: bool,
}

impl Default for CostOptimizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cost_weight: 0.3,
            max_fee_rate_bps: 300,
            optimize_cross_border: true,
            optimize_currency_conversion: true,
        }
    }
}

/// Cost optimizer for gateway selection.
pub struct CostOptimizer {
    config: CostOptimizationConfig,
    fee_structures: std::collections::HashMap<Uuid, GatewayFeeStructure>,
}

impl CostOptimizer {
    pub fn new(config: CostOptimizationConfig) -> Self {
        Self {
            config,
            fee_structures: std::collections::HashMap::new(),
        }
    }

    /// Register fee structure for a gateway.
    pub fn register_fee_structure(&mut self, fee_structure: GatewayFeeStructure) {
        self.fee_structures
            .insert(fee_structure.gateway_id, fee_structure);
    }

    /// Calculate transaction cost for a gateway.
    pub fn calculate_cost(
        &self,
        gateway_id: &Uuid,
        amount_minor: i64,
        currency: &str,
        is_cross_border: bool,
        _target_currency: &str,
    ) -> Option<TransactionCost> {
        let fee_structure = self.fee_structures.get(gateway_id)?;

        let fixed_fee = fee_structure.fixed_fee_minor;

        // Calculate percentage fee
        let percentage_fee = (amount_minor as f64 * fee_structure.percentage_fee_bps as f64
            / 10000.0) as i64;

        // Apply min/max fee constraints
        let percentage_fee = if let Some(min_fee) = fee_structure.min_fee_minor {
            percentage_fee.max(min_fee)
        } else {
            percentage_fee
        };

        let percentage_fee = if let Some(max_fee) = fee_structure.max_fee_minor {
            percentage_fee.min(max_fee)
        } else {
            percentage_fee
        };

        // Cross-border fee
        let cross_border_fee = if is_cross_border && self.config.optimize_cross_border {
            fee_structure
                .cross_border_fee_bps
                .map(|bps| (amount_minor as f64 * bps as f64 / 10000.0) as i64)
                .unwrap_or(0)
        } else {
            0
        };

        // Currency conversion fee
        let currency_conversion_fee = if self.config.optimize_currency_conversion {
            fee_structure
                .currency_conversion_fee_bps
                .map(|bps| (amount_minor as f64 * bps as f64 / 10000.0) as i64)
                .unwrap_or(0)
        } else {
            0
        };

        let total_fee = fixed_fee + percentage_fee + cross_border_fee + currency_conversion_fee;

        // Calculate effective rate in basis points
        let effective_rate_bps = if amount_minor > 0 {
            (total_fee as f64 * 10000.0 / amount_minor as f64) as u32
        } else {
            0
        };

        Some(TransactionCost {
            gateway_id: *gateway_id,
            amount_minor,
            currency: currency.to_string(),
            fixed_fee,
            percentage_fee,
            cross_border_fee,
            currency_conversion_fee,
            total_fee,
            effective_rate_bps,
        })
    }

    /// Select the cheapest gateway.
    pub fn select_cheapest_gateway(
        &self,
        available_gateways: &[Uuid],
        amount_minor: i64,
        currency: &str,
        is_cross_border: bool,
        target_currency: &str,
    ) -> Option<(Uuid, TransactionCost)> {
        if !self.config.enabled {
            return None;
        }

        let mut costs: Vec<(Uuid, TransactionCost)> = available_gateways
            .iter()
            .filter_map(|gw_id| {
                self.calculate_cost(gw_id, amount_minor, currency, is_cross_border, target_currency)
                    .map(|cost| (*gw_id, cost))
            })
            .collect();

        // Sort by total fee (ascending)
        costs.sort_by(|a, b| a.1.total_fee.cmp(&b.1.total_fee));

        costs.first().cloned()
    }

    /// Get fee comparison for all gateways.
    pub fn compare_fees(
        &self,
        available_gateways: &[Uuid],
        amount_minor: i64,
        currency: &str,
        is_cross_border: bool,
        target_currency: &str,
    ) -> Vec<TransactionCost> {
        available_gateways
            .iter()
            .filter_map(|gw_id| {
                self.calculate_cost(gw_id, amount_minor, currency, is_cross_border, target_currency)
            })
            .collect()
    }

    /// Check if gateway is within budget.
    pub fn is_within_budget(
        &self,
        gateway_id: &Uuid,
        amount_minor: i64,
        currency: &str,
        is_cross_border: bool,
        target_currency: &str,
        budget_bps: u32,
    ) -> bool {
        if let Some(cost) =
            self.calculate_cost(gateway_id, amount_minor, currency, is_cross_border, target_currency)
        {
            cost.effective_rate_bps <= budget_bps
        } else {
            false
        }
    }
}

/// Cost tracking for analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrackingEntry {
    pub entry_id: Uuid,
    pub payment_intent_id: Uuid,
    pub gateway_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub estimated_fee: i64,
    pub actual_fee: Option<i64>,
    pub fee_variance: Option<i64>,
    pub recorded_at: DateTime<Utc>,
}

impl CostTrackingEntry {
    pub fn new(
        payment_intent_id: Uuid,
        gateway_id: Uuid,
        amount_minor: i64,
        currency: &str,
        estimated_fee: i64,
    ) -> Self {
        Self {
            entry_id: Uuid::now_v7(),
            payment_intent_id,
            gateway_id,
            amount_minor,
            currency: currency.to_string(),
            estimated_fee,
            actual_fee: None,
            fee_variance: None,
            recorded_at: Utc::now(),
        }
    }

    pub fn record_actual_fee(&mut self, actual_fee: i64) {
        self.actual_fee = Some(actual_fee);
        self.fee_variance = Some(actual_fee - self.estimated_fee);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fee_structure() -> GatewayFeeStructure {
        GatewayFeeStructure {
            gateway_id: Uuid::now_v7(),
            fixed_fee_minor: 30, // $0.30
            percentage_fee_bps: 290, // 2.9%
            cross_border_fee_bps: Some(100), // 1%
            currency_conversion_fee_bps: Some(50), // 0.5%
            min_fee_minor: Some(50), // $0.50
            max_fee_minor: None,
            last_updated: Utc::now(),
        }
    }

    #[test]
    fn test_calculate_cost() {
        let mut optimizer = CostOptimizer::new(CostOptimizationConfig {
            enabled: true,
            ..Default::default()
        });

        let fee_structure = create_test_fee_structure();
        let gateway_id = fee_structure.gateway_id;
        optimizer.register_fee_structure(fee_structure);

        let cost = optimizer.calculate_cost(&gateway_id, 10000, "USD", false, "USD");
        assert!(cost.is_some());

        let cost = cost.unwrap();
        assert_eq!(cost.fixed_fee, 30);
        assert!(cost.percentage_fee > 0);
        assert_eq!(cost.cross_border_fee, 0);
        assert!(cost.total_fee > 0);
    }

    #[test]
    fn test_select_cheapest_gateway() {
        let mut optimizer = CostOptimizer::new(CostOptimizationConfig {
            enabled: true,
            ..Default::default()
        });

        let gw1 = Uuid::now_v7();
        let gw2 = Uuid::now_v7();

        optimizer.register_fee_structure(GatewayFeeStructure {
            gateway_id: gw1,
            fixed_fee_minor: 30,
            percentage_fee_bps: 290,
            cross_border_fee_bps: None,
            currency_conversion_fee_bps: None,
            min_fee_minor: None,
            max_fee_minor: None,
            last_updated: Utc::now(),
        });

        optimizer.register_fee_structure(GatewayFeeStructure {
            gateway_id: gw2,
            fixed_fee_minor: 20,
            percentage_fee_bps: 250,
            cross_border_fee_bps: None,
            currency_conversion_fee_bps: None,
            min_fee_minor: None,
            max_fee_minor: None,
            last_updated: Utc::now(),
        });

        let result = optimizer.select_cheapest_gateway(&[gw1, gw2], 10000, "USD", false, "USD");
        assert!(result.is_some());

        let (cheapest_id, cost) = result.unwrap();
        assert_eq!(cheapest_id, gw2); // gw2 has lower fees
        assert!(cost.total_fee < optimizer.calculate_cost(&gw1, 10000, "USD", false, "USD").unwrap().total_fee);
    }

    #[test]
    fn test_is_within_budget() {
        let mut optimizer = CostOptimizer::new(CostOptimizationConfig {
            enabled: true,
            ..Default::default()
        });

        let gateway_id = Uuid::now_v7();
        optimizer.register_fee_structure(GatewayFeeStructure {
            gateway_id,
            fixed_fee_minor: 30,
            percentage_fee_bps: 290,
            cross_border_fee_bps: None,
            currency_conversion_fee_bps: None,
            min_fee_minor: None,
            max_fee_minor: None,
            last_updated: Utc::now(),
        });

        // 290 bps = 2.9%, budget is 3% (300 bps)
        let within_budget = optimizer.is_within_budget(&gateway_id, 10000, "USD", false, "USD", 300);
        assert!(within_budget);

        // Budget is 2% (200 bps)
        let over_budget = optimizer.is_within_budget(&gateway_id, 10000, "USD", false, "USD", 200);
        assert!(!over_budget);
    }
}
