use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::domain::aggregates::PaymentLink;
use crate::domain::rules::{PaginationParams, PaymentLinkFilter, PaymentLinkRepository};
use crate::domain::value_objects::PaymentLinkStatus;
use crate::infrastructure::entities::payment_link_entity;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct PostgresPaymentLinkRepository {
    db: DatabaseConnection,
}

impl PostgresPaymentLinkRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentLinkRepository for PostgresPaymentLinkRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PaymentLink>, PlatformError> {
        let m = payment_link_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(m.map(|m| m.into()))
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PlatformError> {
        let m = payment_link_entity::Entity::find()
            .filter(payment_link_entity::Column::PublicToken.eq(token))
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;
        Ok(m.map(|m| m.into()))
    }

    async fn save(&self, link: &PaymentLink) -> Result<(), PlatformError> {
        let existing = payment_link_entity::Entity::find_by_id(link.link_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        if let Some(model) = existing {
            let mut a = payment_link_entity::ActiveModel::from(model);
            a.status = Set(link.status.as_str().to_string());
            a.current_uses = Set(link.current_uses);
            a.description = Set(link.description.clone());
            a.merchant_name = Set(link.merchant_name.clone());
            a.updated_at = Set(link.updated_at.into());
            a.update(&self.db).await.map_err(|e| {
                PlatformError::Internal(format!("DB update failed: {e}"))
            })?;
        } else {
            let a = payment_link_entity::ActiveModel {
                link_id: Set(link.link_id),
                operator_id: Set(link.operator_id),
                status: Set(link.status.as_str().to_string()),
                description: Set(link.description.clone()),
                merchant_name: Set(link.merchant_name.clone()),
                amount_minor_units: Set(link.amount.amount_minor_units),
                currency: Set(link.amount.currency.0.clone()),
                max_uses: Set(link.max_uses),
                current_uses: Set(link.current_uses),
                expires_at: Set(link.expires_at.map(|dt| dt.into())),
                public_token: Set(link.public_token.clone()),
                metadata: Set(
                    link.metadata
                        .as_ref()
                        .and_then(|v| serde_json::to_value(v).ok()),
                ),
                created_at: Set(link.created_at.into()),
                updated_at: Set(link.updated_at.into()),
            };
            a.insert(&self.db).await.map_err(|e| {
                PlatformError::Internal(format!("DB insert failed: {e}"))
            })?;
        }
        Ok(())
    }

    async fn list_by_operator(
        &self,
        operator_id: Uuid,
        filter: &PaymentLinkFilter,
        pagination: &PaginationParams,
    ) -> Result<Vec<PaymentLink>, PlatformError> {
        let mut query = payment_link_entity::Entity::find()
            .filter(payment_link_entity::Column::OperatorId.eq(operator_id));

        if let Some(ref status) = filter.status {
            query = query.filter(payment_link_entity::Column::Status.eq(status.as_str()));
        }

        if let Some(min_amount) = filter.min_amount {
            query = query.filter(
                payment_link_entity::Column::AmountMinorUnits.gte(min_amount),
            );
        }

        if let Some(max_amount) = filter.max_amount {
            query = query.filter(
                payment_link_entity::Column::AmountMinorUnits.lte(max_amount),
            );
        }

        let models = query
            .order_by_desc(payment_link_entity::Column::CreatedAt)
            .offset(pagination.offset as u64)
            .limit(pagination.limit as u64)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    async fn find_expired_links(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<PaymentLink>, PlatformError> {
        let models = payment_link_entity::Entity::find()
            .filter(payment_link_entity::Column::Status.eq("active"))
            .filter(payment_link_entity::Column::ExpiresAt.is_not_null())
            .filter(payment_link_entity::Column::ExpiresAt.lte(before))
            .order_by_asc(payment_link_entity::Column::ExpiresAt)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB query failed: {e}")))?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }
}

impl From<payment_link_entity::Model> for PaymentLink {
    fn from(m: payment_link_entity::Model) -> Self {
        PaymentLink {
            link_id: m.link_id,
            operator_id: m.operator_id,
            status: PaymentLinkStatus::from_str(&m.status),
            description: m.description,
            merchant_name: m.merchant_name,
            amount: Money {
                amount_minor_units: m.amount_minor_units,
                currency: CurrencyCode::new(&m.currency).unwrap(),
            },
            max_uses: m.max_uses,
            current_uses: m.current_uses,
            expires_at: m.expires_at.map(|dt| dt.into()),
            public_token: m.public_token,
            metadata: m
                .metadata
                .and_then(|v| serde_json::from_value(v.into()).ok()),
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}
