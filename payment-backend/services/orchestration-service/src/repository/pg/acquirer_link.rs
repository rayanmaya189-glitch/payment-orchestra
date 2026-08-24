//! PostgreSQL-backed AcquirerLinkProvider using raw SQL queries.
//!
//! Queries the `merchant_acquirer_links` table from the merchant-acquirer-link-service
//! to find active acquirer links for a given operator.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use super::PostgresOrchestrationRepository;
use crate::repository::AcquirerLinkProvider;

/// Minimal SeaORM entity for the `merchant_acquirer_links` table.
/// This is a cross-service reference — the full entity is defined in
/// merchant-acquirer-link-service.
mod merchant_acquirer_link {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "merchant_acquirer_links")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub link_id: Uuid,
        pub operator_id: Uuid,
        pub status: String,
        pub connector_id: String,
        pub environment: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[async_trait]
impl AcquirerLinkProvider for PostgresOrchestrationRepository {
    async fn list_active_acquirer_links(&self, operator_id: Uuid) -> Result<Vec<Uuid>, OrchestrationError> {
        use merchant_acquirer_link::{
            Column as MalColumn,
            Entity as MalEntity,
        };

        let results = MalEntity::find()
            .filter(MalColumn::OperatorId.eq(operator_id))
            .filter(MalColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        Ok(results.into_iter().map(|m| m.link_id).collect())
    }

    async fn get_acquirer_link_connector(&self, link_id: Uuid) -> Result<String, OrchestrationError> {
        use merchant_acquirer_link::{
            Column as MalColumn,
            Entity as MalEntity,
        };

        let model = MalEntity::find_by_id(link_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .ok_or_else(|| OrchestrationError::NotFound(link_id))?;

        Ok(model.connector_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquirer_link_reference() {
        let id = Uuid::now_v7();
        use merchant_acquirer_link::Model;
        let _model = Model {
            link_id: id,
            operator_id: Uuid::now_v7(),
            status: "active".into(),
            connector_id: "stripe".into(),
            environment: "sandbox".into(),
        };
    }
}
