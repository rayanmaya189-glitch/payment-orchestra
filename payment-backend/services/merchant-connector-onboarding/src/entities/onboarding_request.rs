//! OnboardingRequest entity — `onboarding_requests` table.
//!
//! Phase 4 additions:
//! - display_name: Human-readable connector name
//! - health_status: Current health status (healthy, degraded, down, unknown)
//! - encrypted_credentials: AES-256-GCM encrypted credentials
//! - last_tested_at: Timestamp of last connection test
//! - credential_expires_at: When credentials expire (for rotation)

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "onboarding_requests")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub onboarding_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub connector_id: String,
    /// Human-readable display name for this connector
    pub display_name: String,
    pub credentials_json: String,
    /// AES-256-GCM encrypted credentials for secure storage
    pub encrypted_credentials: Vec<u8>,
    pub environment: String,
    pub status: String,
    /// Health status: healthy, degraded, down, unknown
    pub health_status: String,
    pub test_result: Option<String>,
    pub error_message: Option<String>,
    /// When credentials were last tested
    pub last_tested_at: Option<DateTimeUtc>,
    /// When credentials expire (for automatic rotation)
    pub credential_expires_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
