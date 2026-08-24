//! Onboarding repository trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;

#[async_trait]
pub trait OnboardingRepository: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<OnboardingRequest>, OnboardingError>;
    async fn save(&self, request: &OnboardingRequest) -> Result<(), OnboardingError>;
    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError>;
    async fn find_active(&self, operator_id: Uuid) -> Result<Vec<OnboardingRequest>, OnboardingError>;
}
