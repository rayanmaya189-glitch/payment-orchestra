//! Gateway Profile repository — trait + in-memory implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::{GatewayProfile, Money, ProfileStatus};

#[async_trait]
pub trait GatewayProfileRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, String>;
    async fn save(&self, profile: &GatewayProfile) -> Result<(), String>;
    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String>;
    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, String>;
    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, String>;
    async fn check_daily_volume(&self, profile_id: Uuid) -> Result<Money, String>;
    async fn check_monthly_volume(&self, profile_id: Uuid) -> Result<Money, String>;
    async fn increment_daily_volume(&self, profile_id: Uuid, amount: Money) -> Result<(), String>;
    async fn get_success_rate(&self, profile_id: Uuid, window_hours: u32) -> Result<f64, String>;
    async fn list_all(&self) -> Result<Vec<GatewayProfile>, String>;
}

pub struct InMemoryGatewayProfileRepository {
    profiles: Arc<RwLock<HashMap<Uuid, GatewayProfile>>>,
    daily_volumes: Arc<RwLock<HashMap<Uuid, i64>>>,
    monthly_volumes: Arc<RwLock<HashMap<Uuid, i64>>>,
    success_counts: Arc<RwLock<HashMap<Uuid, (u32, u32)>>>, // (success, total)
}

impl InMemoryGatewayProfileRepository {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            daily_volumes: Arc::new(RwLock::new(HashMap::new())),
            monthly_volumes: Arc::new(RwLock::new(HashMap::new())),
            success_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_sample_profile(&self, profile: GatewayProfile) {
        let mut store = self.profiles.write().await;
        store.insert(profile.profile_id, profile);
    }
}

impl Default for InMemoryGatewayProfileRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GatewayProfileRepository for InMemoryGatewayProfileRepository {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, String> {
        let store = self.profiles.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn save(&self, profile: &GatewayProfile) -> Result<(), String> {
        let mut store = self.profiles.write().await;
        store.insert(profile.profile_id, profile.clone());
        Ok(())
    }

    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, String> {
        let store = self.profiles.read().await;
        Ok(store
            .values()
            .filter(|p| p.operator_id == operator_id && p.status == ProfileStatus::Active)
            .cloned()
            .collect())
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, String> {
        let store = self.profiles.read().await;
        Ok(store
            .values()
            .filter(|p| p.connector_id == connector_id)
            .cloned()
            .collect())
    }

    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, String> {
        let store = self.profiles.read().await;
        Ok(store.values().find(|p| p.merchant_acquirer_link_id == link_id).cloned())
    }

    async fn check_daily_volume(&self, profile_id: Uuid) -> Result<Money, String> {
        let vols = self.daily_volumes.read().await;
        let amount = vols.get(&profile_id).copied().unwrap_or(0);
        Ok(Money {
            amount_minor_units: amount,
            currency: "AED".into(),
        })
    }

    async fn check_monthly_volume(&self, profile_id: Uuid) -> Result<Money, String> {
        let vols = self.monthly_volumes.read().await;
        let amount = vols.get(&profile_id).copied().unwrap_or(0);
        Ok(Money {
            amount_minor_units: amount,
            currency: "AED".into(),
        })
    }

    async fn increment_daily_volume(&self, profile_id: Uuid, amount: Money) -> Result<(), String> {
        let mut vols = self.daily_volumes.write().await;
        let current = vols.entry(profile_id).or_insert(0);
        *current = current.saturating_add(amount.amount_minor_units);
        Ok(())
    }

    async fn get_success_rate(&self, profile_id: Uuid, _window_hours: u32) -> Result<f64, String> {
        let counts = self.success_counts.read().await;
        match counts.get(&profile_id) {
            Some((success, total)) if *total > 0 => Ok(*success as f64 / *total as f64),
            _ => Ok(1.0), // Default to 100% if no data
        }
    }

    async fn list_all(&self) -> Result<Vec<GatewayProfile>, String> {
        let store = self.profiles.read().await;
        Ok(store.values().cloned().collect())
    }
}
