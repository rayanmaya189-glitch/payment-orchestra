use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
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
    /// Select a gateway profile for a transaction.
    ///
    /// # Deterministic A/B Testing
    ///
    /// When `idempotency_key` is provided, the weighted random selection uses
    /// a seeded RNG derived from the key, ensuring the same transaction always
    /// routes to the same gateway. This enables reproducible A/B test results
    /// and consistent retry behavior.
    ///
    /// When `idempotency_key` is `None`, a thread-local RNG is used (non-deterministic).
    pub fn select_gateway_profile(
        &self,
        profiles: &[GatewayProfile],
        amount: &Money,
        card_scheme: &CardScheme,
        currency: &str,
        idempotency_key: Option<&str>,
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

                // Use seeded RNG for deterministic A/B testing when idempotency_key is provided
                let mut rng = match idempotency_key {
                    Some(key) => {
                        // Create a deterministic seed using FNV-1a hash over the full key
                        // for uniform distribution regardless of key prefix patterns
                        let seed = fnv_hash(key.as_bytes());
                        StdRng::seed_from_u64(seed)
                    }
                    None => StdRng::from_entropy(),
                };

                let mut random = rng.gen_range(0..total_weight);
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

/// FNV-1a hash for deterministic seed generation.
///
/// FNV-1a is chosen for its excellent distribution properties and speed.
/// It provides uniform hash values even for keys with common prefixes
/// (e.g., `sk_test_...` style API keys).
fn fnv_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
