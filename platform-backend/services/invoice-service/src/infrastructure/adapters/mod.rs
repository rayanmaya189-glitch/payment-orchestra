use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use uuid::Uuid;

use crate::domain::aggregates::Invoice;
use crate::infrastructure::entities::invoice;
use crate::infrastructure::repository::InvoiceRepository;
use platform_error::PlatformError;

pub struct PostgresInvoiceRepository {
    db: DatabaseConnection,
}

impl PostgresInvoiceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl InvoiceRepository for PostgresInvoiceRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Invoice>, PlatformError> {
        let model = invoice::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn save(&self, inv: &Invoice) -> Result<(), PlatformError> {
        let active_model: invoice::ActiveModel = inv.clone().into();

        let existing = invoice::Entity::find_by_id(inv.invoice_id)
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

    async fn find_by_order_reference(&self, operator_id: Uuid, order_ref: &str) -> Result<Option<Invoice>, PlatformError> {
        let model = invoice::Entity::find()
            .filter(invoice::Column::OperatorId.eq(operator_id))
            .filter(invoice::Column::OrderReference.eq(order_ref))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        Ok(model.map(|m| m.to_domain()))
    }

    async fn find_by_payment_intent(&self, payment_intent_id: Uuid) -> Result<Option<Invoice>, PlatformError> {
        // Search in JSON array of payment_intent_ids
        let models = invoice::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("Database error: {e}")))?;

        for model in models {
            let ids: Vec<Uuid> = serde_json::from_str(&model.payment_intent_ids).unwrap_or_default();
            if ids.contains(&payment_intent_id) {
                return Ok(Some(model.to_domain()));
            }
        }

        Ok(None)
    }
}
