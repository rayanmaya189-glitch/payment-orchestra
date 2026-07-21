pub mod network_international;
pub mod fawry;
pub mod tap;
pub mod hyperpay;


use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, QueryOrder};
use uuid::Uuid;

use crate::domain::aggregates::GatewayProfile;
use crate::infrastructure::entities::gateway_profile;
use crate::infrastructure::repository::GatewayProfileRepository;
use platform_error::PlatformError;

pub struct PostgresGatewayProfileRepository {
    db: DatabaseConnection,
}

impl PostgresGatewayProfileRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GatewayProfileRepository for PostgresGatewayProfileRepository {
    async fn load(&self, id: Uuid) -> Result<Option<GatewayProfile>, PlatformError> {
        let model = gateway_profile::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, profile: &GatewayProfile) -> Result<(), PlatformError> {
        let active_model: gateway_profile::ActiveModel = profile.clone().into();

        let existing = gateway_profile::Entity::find_by_id(profile.profile_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        match existing {
            Some(_) => {
                active_model
                    .update(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
            None => {
                active_model
                    .insert(&self.db)
                    .await
                    .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;
            }
        }

        Ok(())
    }

    async fn find_active_for_operator(&self, operator_id: Uuid) -> Result<Vec<GatewayProfile>, PlatformError> {
        let models = gateway_profile::Entity::find()
            .filter(gateway_profile::Column::OperatorId.eq(operator_id))
            .filter(gateway_profile::Column::Status.eq("active"))
            .order_by_asc(gateway_profile::Column::RoutingPriority)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }

    async fn find_by_connector(&self, connector_id: &str) -> Result<Vec<GatewayProfile>, PlatformError> {
        let models = gateway_profile::Entity::find()
            .filter(gateway_profile::Column::ConnectorId.eq(connector_id))
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(models.into_iter().map(|m| m.to_domain()).collect())
    }

    async fn find_by_link(&self, link_id: Uuid) -> Result<Option<GatewayProfile>, PlatformError> {
        let model = gateway_profile::Entity::find()
            .filter(gateway_profile::Column::MerchantAcquirerLinkId.eq(link_id))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }
}
