//! PostgreSQL-backed PaymentMethodTokenRepository using SeaORM CRUD.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::payment_method_token::{
    ActiveModel as PaymentMethodTokenActiveModel, Column as PaymentMethodTokenColumn,
    Entity as PaymentMethodTokenEntity, Model as PaymentMethodTokenModel,
};
use super::PostgresOrchestrationRepository;
use crate::repository::PaymentMethodTokenRepository;

#[async_trait]
impl PaymentMethodTokenRepository for PostgresOrchestrationRepository {
    async fn load_payment_method_token(&self, id: Uuid) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        let result = PaymentMethodTokenEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save_payment_method_token(&self, token: &PaymentMethodToken) -> Result<(), OrchestrationError> {
        let model = PaymentMethodTokenModel {
            token_id: token.token_id,
            operator_id: token.operator_id,
            token_status: match token.token_status {
                TokenStatus::Active => "Active".to_string(),
                TokenStatus::Expired => "Expired".to_string(),
                TokenStatus::Revoked => "Revoked".to_string(),
            },
            payment_method_type: token.payment_method_type.clone(),
            token_ref: token.acquirer_token_reference.clone(),
            created_at: token.created_at,
            expires_at: token.expires_at,
        };

        let exists = PaymentMethodTokenEntity::find_by_id(token.token_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            PaymentMethodTokenEntity::update(PaymentMethodTokenActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        } else {
            PaymentMethodTokenEntity::insert(PaymentMethodTokenActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_active_tokens_for_operator(&self, operator_id: Uuid) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        let models = PaymentMethodTokenEntity::find()
            .filter(PaymentMethodTokenColumn::OperatorId.eq(operator_id))
            .filter(PaymentMethodTokenColumn::TokenStatus.eq("Active"))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;

        models.into_iter().map(model_to_domain).collect()
    }
}

fn model_to_domain(m: PaymentMethodTokenModel) -> Result<PaymentMethodToken, OrchestrationError> {
    Ok(PaymentMethodToken {
        token_id: m.token_id,
        operator_id: m.operator_id,
        payment_method_type: m.payment_method_type,
        last_four: String::new(),
        card_brand: None,
        expiry_month: None,
        expiry_year: None,
        token_status: match m.token_status.as_str() {
            "Active" => TokenStatus::Active,
            "Expired" => TokenStatus::Expired,
            "Revoked" => TokenStatus::Revoked,
            _ => TokenStatus::Active,
        },
        acquirer_link_id: Uuid::nil(),
        acquirer_token_reference: m.token_ref,
        encrypted_token: Vec::new(),
        created_at: m.created_at,
        expires_at: m.expires_at,
        revoked_at: None,
        revocation_reason: None,
    })
}
