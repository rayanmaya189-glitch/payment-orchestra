//! PostgreSQL-backed PaymentLinkRepository using SeaORM CRUD.
//!
//! NOTE: Entity field name/type mismatches with domain model:
//! - entity `link_id` ←→ domain `payment_link_id`
//! - entity `checkout_token` ←→ domain `token`
//! - entity `payment_intent_ids` (Json) ←→ domain `payment_intent_id` (Option<Uuid>)
//!   Missing entity columns default: max_uses=0, use_count=0, success_url="", cancel_url=""

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::payment_link::{
    ActiveModel as PaymentLinkActiveModel, Column as PaymentLinkColumn,
    Entity as PaymentLinkEntity, Model as PaymentLinkModel,
};

#[derive(Clone)]
pub struct PostgresPaymentLinkRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresPaymentLinkRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn domain_to_model(link: &PaymentLink) -> Result<PaymentLinkModel, PaymentLinkError> {
    let payment_intent_ids = match link.payment_intent_id {
        Some(id) => serde_json::to_value(vec![id])
            .map_err(|_| PaymentLinkError::InvalidLinkAmount)?,
        None => serde_json::Value::Array(vec![]),
    };

    Ok(PaymentLinkModel {
        link_id: link.payment_link_id,
        operator_id: link.operator_id,
        amount_minor_units: link.amount_minor_units,
        currency: link.currency.clone(),
        description: link.description.clone().unwrap_or_default(),
        status: link.status.to_string(),
        checkout_token: link.token.clone(),
        expires_at: link.expires_at,
        max_uses: 0,
        use_count: 0,
        success_url: String::new(),
        cancel_url: String::new(),
        payment_intent_ids,
        created_at: link.created_at,
        updated_at: Utc::now(),
    })
}

fn model_to_domain(m: PaymentLinkModel) -> Result<PaymentLink, PaymentLinkError> {
    let status: PaymentLinkStatus = m.status
        .parse()
        .map_err(|_e: String| PaymentLinkError::InvalidTransition)?;

    // Extract first payment_intent_id from JSONB array
    let payment_intent_id = match serde_json::from_value::<Vec<Uuid>>(m.payment_intent_ids) {
        Ok(ids) => ids.into_iter().next(),
        Err(_) => None,
    };

    Ok(PaymentLink {
        payment_link_id: m.link_id,
        operator_id: m.operator_id,
        token: m.checkout_token,
        status,
        amount_minor_units: m.amount_minor_units,
        currency: m.currency,
        description: if m.description.is_empty() { None } else { Some(m.description) },
        invoice_id: None,
        expires_at: m.expires_at,
        used_at: None,
        payment_intent_id,
        created_at: m.created_at,
    })
}

// ─── PaymentLinkRepository Trait Implementation ──────────────────────────────

use crate::repository::PaymentLinkRepository;

#[async_trait]
impl PaymentLinkRepository for PostgresPaymentLinkRepository {
    async fn load(&self, id: Uuid) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let result = PaymentLinkEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|_| PaymentLinkError::InvalidTransition)?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn load_by_token(&self, token: &str) -> Result<Option<PaymentLink>, PaymentLinkError> {
        let result = PaymentLinkEntity::find()
            .filter(PaymentLinkColumn::CheckoutToken.eq(token))
            .one(&self.db)
            .await
            .map_err(|_| PaymentLinkError::InvalidTransition)?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, link: &PaymentLink) -> Result<(), PaymentLinkError> {
        let model = domain_to_model(link)?;

        let exists = PaymentLinkEntity::find_by_id(link.payment_link_id)
            .one(&self.db)
            .await
            .map_err(|_| PaymentLinkError::InvalidTransition)?
            .is_some();

        if exists {
            PaymentLinkEntity::update(PaymentLinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|_| PaymentLinkError::InvalidTransition)?;
        } else {
            PaymentLinkEntity::insert(PaymentLinkActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|_| PaymentLinkError::InvalidTransition)?;
        }

        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let results = PaymentLinkEntity::find()
            .filter(PaymentLinkColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|_| PaymentLinkError::InvalidTransition)?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn find_expired(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let results = PaymentLinkEntity::find()
            .filter(PaymentLinkColumn::ExpiresAt.lt(Utc::now()))
            .filter(PaymentLinkColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|_| PaymentLinkError::InvalidTransition)?;

        results.into_iter().map(model_to_domain).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_link_domain_entity_roundtrip() {
        let link = PaymentLink::new(
            Uuid::now_v7(),
            "plink_test123abc".into(),
            5000,
            "AED".into(),
            Some("Test payment".into()),
            None,
            Utc::now() + chrono::Duration::days(30),
        ).unwrap();

        let model = domain_to_model(&link).unwrap();
        let roundtrip = model_to_domain(model).unwrap();

        assert_eq!(roundtrip.payment_link_id, link.payment_link_id);
        assert_eq!(roundtrip.token, "plink_test123abc");
        assert_eq!(roundtrip.amount_minor_units, 5000);
        assert_eq!(roundtrip.currency, "AED");
        assert_eq!(roundtrip.description, Some("Test payment".into()));
        assert_eq!(roundtrip.status, PaymentLinkStatus::Active);
        assert!(roundtrip.payment_intent_id.is_none());
    }
}
