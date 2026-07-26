//! Authentication command handler.

use chrono::Utc;
use tracing::info;

use uuid::Uuid;
use crate::domain::{AuthError, IamError};
use super::handler::{ACCESS_TOKEN_TTL_SECS, REFRESH_TOKEN_TTL_SECS};
use crate::events::{IamEvent, PrincipalAuthenticated, PermissionDenied};
use crate::repository::IamRepository;
use super::types::*;
use super::IamCommandHandler;

impl<R: IamRepository + Send + Sync> IamCommandHandler<R> {
    pub(crate) async fn authenticate_impl(&self, cmd: Authenticate) -> Result<AuthenticateResult, IamError> {
        let principal = self.repository.find_principal_by_email(&cmd.email).await?
            .ok_or(AuthError::InvalidCredentials)?;

        // Check account status
        principal.can_authenticate()?;

        // Verify password against Argon2id hash
        let password_valid = self.verify_password(&cmd.password, principal.password_hash.as_deref().unwrap_or_default());
        if !password_valid {
            let mut p = principal.clone();
            // Save first, then check lock — ensures locked state is persisted
            // before propagating any lock error to the caller
            let login_result = p.record_login_attempt(false);
            self.repository.save_principal(&p).await?;

            if let Err(e) = login_result {
                // Account is now locked — return the lock error
                return Err(e.into());
            }

            self.publish_event(IamEvent::PermissionDenied(PermissionDenied {
                principal_id: p.id,
                resource: "authentication".into(),
                action: "login".into(),
                reason: "Invalid credentials".into(),
                occurred_at: Utc::now(),
            }));
            return Err(AuthError::InvalidCredentials.into());
        }

        // Record successful login
        let mut p = principal.clone();
        p.record_login_attempt(true)?;
        self.repository.save_principal(&p).await?;

        // Generate tokens using proper JWT with Argon2id-secured credentials
        let permissions: Vec<String> = vec![]; // TODO: load permissions from principal roles
        let operator_id: Option<Uuid> = None; // TODO: load operator context
        let access_token = self.generate_token(
            &p.id,
            p.principal_type.as_str(),
            "access",
            ACCESS_TOKEN_TTL_SECS,
            &permissions,
            operator_id,
        )?;
        let refresh_token = self.generate_token(
            &p.id,
            p.principal_type.as_str(),
            "refresh",
            REFRESH_TOKEN_TTL_SECS,
            &permissions,
            operator_id,
        )?;

        self.publish_event(IamEvent::PrincipalAuthenticated(PrincipalAuthenticated {
            principal_id: p.id,
            ip_address: cmd.ip_address.to_string(),
            user_agent: cmd.user_agent,
            occurred_at: Utc::now(),
        }));

        info!(principal_id = %p.id, "Principal authenticated");

        Ok(AuthenticateResult {
            principal: p,
            access_token,
            refresh_token,
            mfa_required: principal.mfa_enrolled,
            mfa_method: principal.mfa_method.as_ref().map(|m| m.as_str().to_string()),
        })
    }
}
