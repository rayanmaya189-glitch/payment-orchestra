//! Payment method token command handlers.

use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use super::types::*;
use super::OrchestrationCommandHandler;

impl<R: OrchestrationRepository + Send + Sync> OrchestrationCommandHandler<R> {
    pub(crate) async fn store_payment_method_token_impl(&self, cmd: StorePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        let token_id = Uuid::now_v7();
        let token = PaymentMethodToken {
            token_id,
            operator_id: cmd.operator_id,
            payment_method_type: cmd.payment_method_type.clone(),
            last_four: cmd.last_four.clone(),
            card_brand: cmd.card_brand.clone(),
            expiry_month: cmd.expiry_month,
            expiry_year: cmd.expiry_year,
            token_status: TokenStatus::Active,
            acquirer_link_id: cmd.acquirer_link_id,
            acquirer_token_reference: cmd.acquirer_token_reference.clone(),
            encrypted_token: cmd.encrypted_token.clone(),
            created_at: Utc::now(),
            expires_at: cmd.expires_at,
            revoked_at: None,
            revocation_reason: None,
        };

        let event = PaymentEvent::PaymentMethodTokenStored(PaymentMethodTokenStored {
            token_id,
            operator_id: cmd.operator_id,
            payment_method_type: cmd.payment_method_type,
            last_four: cmd.last_four,
            card_brand: cmd.card_brand,
            acquirer_link_id: cmd.acquirer_link_id,
            occurred_at: Utc::now(),
        });

        self.repo.save_payment_method_token(&token).await?;

        Ok(TokenResult { token_id, event })
    }

    pub(crate) async fn expire_payment_method_token_impl(&self, cmd: ExpirePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        let mut token = self.repo.load_payment_method_token(cmd.token_id).await?
            .ok_or(OrchestrationError::PaymentMethodTokenInvalid)?;
        token.token_status = TokenStatus::Expired;

        let event = PaymentEvent::PaymentMethodTokenExpired(PaymentMethodTokenExpired {
            token_id: cmd.token_id,
            last_four: token.last_four.clone(),
            acquirer_link_id: token.acquirer_link_id,
            occurred_at: Utc::now(),
        });

        self.repo.save_payment_method_token(&token).await?;

        Ok(TokenResult { token_id: cmd.token_id, event })
    }

    pub(crate) async fn revoke_payment_method_token_impl(&self, cmd: RevokePaymentMethodToken) -> Result<TokenResult, OrchestrationError> {
        let mut token = self.repo.load_payment_method_token(cmd.token_id).await?
            .ok_or(OrchestrationError::PaymentMethodTokenInvalid)?;
        token.token_status = TokenStatus::Revoked;
        token.revoked_at = Some(Utc::now());
        token.revocation_reason = cmd.reason.clone();

        let event = PaymentEvent::PaymentMethodTokenRevoked(PaymentMethodTokenRevoked {
            token_id: cmd.token_id,
            last_four: token.last_four.clone(),
            acquirer_link_id: token.acquirer_link_id,
            revocation_reason: cmd.reason,
            occurred_at: Utc::now(),
        });

        self.repo.save_payment_method_token(&token).await?;

        Ok(TokenResult { token_id: cmd.token_id, event })
    }
}
