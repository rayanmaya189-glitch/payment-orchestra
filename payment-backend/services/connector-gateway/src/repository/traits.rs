//! Gateway Profile repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{GatewayProfile, Money};

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
