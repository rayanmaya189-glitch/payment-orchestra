use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresOrchestrationRepository;
use crate::repository::PaymentMethodTokenRepository;
use crate::domain::*;
use crate::entities::payment_method_token::{
    Entity as PaymentMethodTokenEntity,
    ActiveModel as PaymentMethodTokenActiveModel,
    Model as PaymentMethodTokenModel,
    Column as PaymentMethodTokenColumn,
};

#[async_trait]
impl PaymentMethodTokenRepository for PostgresOrchestrationRepository {
    async fn load_payment_method_token(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentMethodToken>, OrchestrationError> {
        let result = PaymentMethodTokenEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(payment_method_token_model_to_domain(m))),
            None => Ok(None),
        }
    }

    async fn save_payment_method_token(
        &self,
        token: &PaymentMethodToken,
    ) -> Result<(), OrchestrationError> {
        let model = payment_method_token_domain_to_model(token);
        let exists = PaymentMethodTokenEntity::find_by_id(token.token_id)
            .one(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            PaymentMethodTokenEntity::update(PaymentMethodTokenActiveModel::from(model.clone()))
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

    async fn find_active_tokens_for_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<PaymentMethodToken>, OrchestrationError> {
        let models = PaymentMethodTokenEntity::find()
            .filter(PaymentMethodTokenColumn::OperatorId.eq(operator_id))
            .filter(PaymentMethodTokenColumn::TokenStatus.eq("Active"))
            .all(&self.db)
            .await
            .map_err(|e| OrchestrationError::DatabaseError(e.to_string()))?;
        Ok(models.into_iter().map(payment_method_token_model_to_domain).collect())
    }
}

// ─── Domain ↔ Model conversion: PaymentMethodToken ───────────────────────────

fn payment_method_token_domain_to_model(
    t: &PaymentMethodToken,
) -> PaymentMethodTokenModel {
    PaymentMethodTokenModel {
        token_id: t.token_id,
        operator_id: t.operator_id,
        token_status: format!("{:?}", t.token_status),
        payment_method_type: t.payment_method_type.clone(),
        token_ref: t.acquirer_token_reference.clone(),
        created_at: t.created_at,
        expires_at: t.expires_at,
    }
}

fn payment_method_token_model_to_domain(
    m: PaymentMethodTokenModel,
) -> PaymentMethodToken {
    let token_status = match m.token_status.as_str() {
        "Active" => TokenStatus::Active,
        "Expired" => TokenStatus::Expired,
        "Revoked" => TokenStatus::Revoked,
        _ => TokenStatus::Active,
    };

    PaymentMethodToken {
        token_id: m.token_id,
        operator_id: m.operator_id,
        payment_method_type: m.payment_method_type,
        last_four: String::new(),
        card_brand: None,
        expiry_month: None,
        expiry_year: None,
        token_status,
        acquirer_link_id: Uuid::default(),
        acquirer_token_reference: m.token_ref,
        encrypted_token: Vec::new(),
        created_at: m.created_at,
        expires_at: m.expires_at,
        revoked_at: None,
        revocation_reason: None,
    }
}
