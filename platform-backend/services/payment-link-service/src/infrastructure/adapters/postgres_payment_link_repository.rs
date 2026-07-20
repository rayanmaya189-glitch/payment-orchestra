use async_trait::async_trait;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;
use crate::domain::aggregates::PaymentLink;
use crate::domain::value_objects::PaymentLinkStatus;
use crate::domain::rules::PaymentLinkRepository;
use crate::infrastructure::entities::payment_link_entity;
use platform_error::PlatformError;
use shared_types::{CurrencyCode, Money};

pub struct PostgresPaymentLinkRepository { db: DatabaseConnection }
impl PostgresPaymentLinkRepository { pub fn new(db: DatabaseConnection) -> Self { Self { db } } }

#[async_trait]
impl PaymentLinkRepository for PostgresPaymentLinkRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PaymentLink>, PlatformError> {
        let m = payment_link_entity::Entity::find_by_id(id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn find_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PlatformError> {
        let m = payment_link_entity::Entity::find().filter(payment_link_entity::Column::PublicToken.eq(token))
            .one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        Ok(m.map(|m| m.into()))
    }
    async fn save(&self, link: &PaymentLink) -> Result<(), PlatformError> {
        let existing = payment_link_entity::Entity::find_by_id(link.link_id).one(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        if let Some(model) = existing {
            let mut a = payment_link_entity::ActiveModel::from(model);
            a.status = Set(link.status.as_str().to_string());
            a.current_uses = Set(link.current_uses);
            a.updated_at = Set(link.updated_at.into());
            a.update(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        } else {
            let a = payment_link_entity::ActiveModel {
                link_id: Set(link.link_id), operator_id: Set(link.operator_id),
                status: Set(link.status.as_str().to_string()),
                description: Set(link.description.clone()), merchant_name: Set(link.merchant_name.clone()),
                amount_minor_units: Set(link.amount.amount_minor_units), currency: Set(link.amount.currency.0.clone()),
                max_uses: Set(link.max_uses), current_uses: Set(link.current_uses),
                expires_at: Set(link.expires_at.map(|dt| dt.into())),
                public_token: Set(link.public_token.clone()),
                metadata: Set(link.metadata.as_ref().and_then(|v| serde_json::to_value(v).ok())),
                created_at: Set(link.created_at.into()), updated_at: Set(link.updated_at.into()),
            };
            a.insert(&self.db).await.map_err(|e| PlatformError::Internal(format!("DB: {e}")))?;
        }
        Ok(())
    }
}

impl From<payment_link_entity::Model> for PaymentLink {
    fn from(m: payment_link_entity::Model) -> Self {
        PaymentLink {
            link_id: m.link_id, operator_id: m.operator_id,
            status: PaymentLinkStatus::from_str(&m.status),
            description: m.description, merchant_name: m.merchant_name,
            amount: Money { amount_minor_units: m.amount_minor_units, currency: CurrencyCode::new(&m.currency).unwrap() },
            max_uses: m.max_uses, current_uses: m.current_uses,
            expires_at: m.expires_at.map(|dt| dt.into()),
            public_token: m.public_token,
            metadata: m.metadata.and_then(|v| serde_json::from_value(v.into()).ok()),
            created_at: m.created_at.into(), updated_at: m.updated_at.into(),
        }
    }
}
