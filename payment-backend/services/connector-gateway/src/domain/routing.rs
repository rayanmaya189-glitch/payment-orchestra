use std::collections::HashMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::ConnectorError;
use super::gateway_profile::{GatewayProfile, ProfileStatus};
use super::types::{CardScheme, Money};

/// Strategy for selecting which gateway profile to use for a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    Priority,
    RoundRobin,
    WeightedRoundRobin { weights: Vec<(Uuid, u32)> },
    CostBased,
    SuccessRateBased,
    VolumeCapped,
}

/// Stateful context for gateway profile rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationState {
    pub operator_id: Uuid,
    pub strategy: RotationStrategy,
    pub current_index: u32,
    pub last_used_gateway_id: Option<Uuid>,
    pub weights: Vec<(Uuid, u32)>,
    pub daily_volume: HashMap<Uuid, i64>,
}

impl RotationState {
    pub fn select_gateway_profile(
        &self,
        profiles: &[GatewayProfile],
        amount: &Money,
        card_scheme: &CardScheme,
        currency: &str,
    ) -> Result<Uuid, ConnectorError> {
        let eligible: Vec<&GatewayProfile> = profiles
            .iter()
            .filter(|p| p.status == ProfileStatus::Active)
            .filter(|p| p.enabled_card_schemes.contains(card_scheme))
            .filter(|p| p.enabled_currencies.iter().any(|c| c == currency))
            .filter(|p| amount.amount_minor_units >= p.limits.min_amount_minor)
            .filter(|p| amount.amount_minor_units <= p.limits.max_amount_minor)
            .filter(|p| {
                let daily = self.daily_volume.get(&p.profile_id).copied().unwrap_or(0);
                daily < p.limits.daily_volume_limit_minor
            })
            .collect();

        if eligible.is_empty() {
            return Err(ConnectorError::InvalidRequest("No eligible gateway profile found".into()));
        }

        match &self.strategy {
            RotationStrategy::Priority => Ok(eligible[0].profile_id),
            RotationStrategy::RoundRobin => {
                let idx = (self.current_index as usize) % eligible.len();
                Ok(eligible[idx].profile_id)
            }
            RotationStrategy::WeightedRoundRobin { weights } => {
                let total_weight: u32 = weights.iter().map(|(_, w)| w).sum();
                if total_weight == 0 {
                    return Ok(eligible[0].profile_id);
                }
                let mut random = rand::thread_rng().gen_range(0..total_weight);
                for (gateway_id, weight) in weights {
                    if random < *weight {
                        return Ok(*gateway_id);
                    }
                    random = random.saturating_sub(*weight);
                }
                Ok(eligible[0].profile_id) // fallback
            }
            RotationStrategy::CostBased => {
                let mut scored: Vec<(Uuid, i64)> = eligible
                    .iter()
                    .map(|p| {
                        let fee = p.fees.calculate_fee(amount, false, false, 0);
                        (p.profile_id, fee.amount_minor_units)
                    })
                    .collect();
                scored.sort_by_key(|(_, fee)| *fee);
                Ok(scored[0].0)
            }
            RotationStrategy::SuccessRateBased => {
                // Without real success rates, fall back to priority
                Ok(eligible[0].profile_id)
            }
            RotationStrategy::VolumeCapped => {
                for profile in &eligible {
                    let daily = self.daily_volume.get(&profile.profile_id).copied().unwrap_or(0);
                    if daily < profile.limits.daily_volume_limit_minor {
                        return Ok(profile.profile_id);
                    }
                }
                Err(ConnectorError::InvalidRequest("All gateways exceeded volume limits".into()))
            }
        }
    }
}
