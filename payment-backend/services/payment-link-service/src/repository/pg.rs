//! PostgreSQL-backed PaymentLinkRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::PaymentLinkRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as PaymentLinkActiveModel,
    Column as PaymentLinkColumn,
    Entity as PaymentLinkEntity,
    Model as PaymentLinkModel,
};

pub struct PostgresPaymentLinkRepository {
    pub db: DatabaseConnection,
}

impl PostgresPaymentLinkRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentLinkRepository for PostgresPaymentLinkRepository {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let result = PaymentLinkEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PaymentLinkError::NotFound)?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn load_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let result = PaymentLinkEntity::find()
            .filter(PaymentLinkColumn::CheckoutToken.eq(token))
            .one(&self.db)
            .await
            .map_err(|e| PaymentLinkError::NotFound)?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, link: &PaymentLink) -> Result<(), PaymentLinkError> {
        let model = domain_to_model(link);
        let exists = PaymentLinkEntity::find_by_id(link.link_id)
            .one(&self.db)
            .await
            .map_err(|e| PaymentLinkError::NotFound)?
            .is_some();

        if exists {
            PaymentLinkEntity::update(PaymentLinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| PaymentLinkError::NotFound)?;
        } else {
            PaymentLinkEntity::insert(PaymentLinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| PaymentLinkError::NotFound)?;
        }
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let models = PaymentLinkEntity::find()
            .filter(PaymentLinkColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| PaymentLinkError::NotFound)?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_expired(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let now = Utc::now();
        let models = PaymentLinkEntity::find()
            .filter(PaymentLinkColumn::ExpiresAt.lt(now))
            .filter(PaymentLinkColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| PaymentLinkError::NotFound)?;
        models.into_iter().map(model_to_domain).collect()
    }
}

fn domain_to_model(l: &PaymentLink) -> PaymentLinkModel {
    let payment_intent_ids = serde_json::to_value(&l.payment_intent_ids).unwrap_or_default();
    PaymentLinkModel {
        link_id: l.link_id,
        operator_id: l.operator_id,
        amount_minor_units: l.amount_minor_units,
        currency: l.currency.clone(),
        description: l.description.clone(),
        status: l.status.as_str().to_string(),
        checkout_token: l.checkout_token.clone(),
        expires_at: l.expires_at,
        max_uses: l.max_uses as i32,
        use_count: l.use_count as i32,
        success_url: l.success_url.clone(),
        cancel_url: l.cancel_url.clone(),
        payment_intent_ids,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }
}

fn model_to_domain(m: PaymentLinkModel) -> Result<PaymentLink, PaymentLinkError> {
    let status = PaymentLinkStatus::from_str(&m.status)
        .ok_or_else(|| PaymentLinkError::NotFound)?;
    let payment_intent_ids: Vec<Uuid> = serde_json::from_value(m.payment_intent_ids)
        .unwrap_or_default();

    Ok(PaymentLink {
        link_id: m.link_id,
        operator_id: m.operator_id,
        amount_minor_units: m.amount_minor_units,
        currency: m.currency,
        description: m.description,
        status,
        checkout_token: m.checkout_token,
        expires_at: m.expires_at,
        max_uses: m.max_uses as u32,
        use_count: m.use_count as u32,
        success_url: m.success_url,
        cancel_url: m.cancel_url,
        payment_intent_ids,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
