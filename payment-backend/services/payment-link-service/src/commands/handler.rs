//! Payment Link command handlers — BC-07

use async_trait::async_trait;
use chrono::{Duration, Utc};
use rand::Rng;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default expiry for a payment link (30 days).
const DEFAULT_EXPIRY_DAYS: u32 = 30;

/// Base62 alphabet for token generation.
const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn create_payment_link(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError>;
    async fn resolve_payment_link(&self, cmd: ResolvePaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError>;
    async fn cancel_payment_link(&self, cmd: CancelPaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError>;
    async fn expire_overdue_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError>;
}

pub struct PaymentLinkCommandHandler<R: PaymentLinkRepository> {
    repo: R,
}

impl<R: PaymentLinkRepository> PaymentLinkCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Generate a cryptographically-secure token.
    /// Format: `plink_` + 22 chars of base62 (132 bits of entropy).
    fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let mut token = String::with_capacity(28);
        token.push_str("plink_");
        for _ in 0..22 {
            let idx = rng.r#gen::<u8>() as usize % BASE62.len();
            token.push(BASE62[idx] as char);
        }
        token
    }
}

#[async_trait]
impl<R: PaymentLinkRepository + Send + Sync> CommandHandler for PaymentLinkCommandHandler<R> {
    async fn create_payment_link(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError> {
        if cmd.amount_minor_units <= 0 {
            return Err(PaymentLinkError::InvalidLinkAmount);
        }

        let expires_at = Utc::now()
            + Duration::days(i64::from(cmd.expires_in_days.unwrap_or(DEFAULT_EXPIRY_DAYS)));

        let token = Self::generate_token();
        let link = PaymentLink::new(
            cmd.operator_id,
            token,
            cmd.amount_minor_units,
            cmd.currency,
            cmd.description,
            cmd.invoice_id,
            expires_at,
        )?;

        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn resolve_payment_link(&self, cmd: ResolvePaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError> {
        let mut link = self.repo.load_by_token(&cmd.token).await?
            .ok_or(PaymentLinkError::NotFound)?;

        if link.is_expired(&Utc::now()) {
            return Err(PaymentLinkError::LinkExpired);
        }

        link.mark_used(cmd.payment_intent_id)?;
        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn cancel_payment_link(&self, cmd: CancelPaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError> {
        let mut link = self.repo.load(cmd.payment_link_id).await?
            .ok_or(PaymentLinkError::NotFound)?;

        if link.status != PaymentLinkStatus::Active {
            return Err(match link.status {
                PaymentLinkStatus::Used => PaymentLinkError::LinkAlreadyUsed,
                PaymentLinkStatus::Expired => PaymentLinkError::LinkExpired,
                PaymentLinkStatus::Cancelled => PaymentLinkError::LinkCancelled,
                _ => PaymentLinkError::InvalidTransition,
            });
        }

        link.status = PaymentLinkStatus::Cancelled;
        self.repo.save(&link).await?;
        Ok(link)
    }

    async fn expire_overdue_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        let overdue = self.repo.find_expired().await?;
        let mut expired = Vec::new();

        for mut link in overdue {
            if link.status == PaymentLinkStatus::Active && link.is_expired(&Utc::now()) {
                link.mark_expired().ok();
                self.repo.save(&link).await?;
                expired.push(link);
            }
        }

        Ok(expired)
    }
}

// ─── Blanket impl: Box<dyn CommandHandler> delegates to inner ────────────────

#[async_trait]
impl<T: CommandHandler + ?Sized> CommandHandler for Box<T> {
    async fn create_payment_link(&self, cmd: CreatePaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError> {
        (**self).create_payment_link(cmd).await
    }

    async fn resolve_payment_link(&self, cmd: ResolvePaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError> {
        (**self).resolve_payment_link(cmd).await
    }

    async fn cancel_payment_link(&self, cmd: CancelPaymentLinkCommand) -> Result<PaymentLink, PaymentLinkError> {
        (**self).cancel_payment_link(cmd).await
    }

    async fn expire_overdue_links(&self) -> Result<Vec<PaymentLink>, PaymentLinkError> {
        (**self).expire_overdue_links().await
    }
}
