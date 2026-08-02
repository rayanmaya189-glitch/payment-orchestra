//! PostgreSQL-backed Principal operations using SeaORM CRUD.
//!
//! Converts between the domain Principal model (with PrincipalType, MfaMethod,
//! PrincipalStatus enums) and the flat SeaORM entity model (principals table).

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::{IamError, MfaMethod, Principal, PrincipalStatus, PrincipalType};
use crate::entities::principal::{
    ActiveModel as PrincipalActiveModel, Column as PrincipalColumn,
    Entity as PrincipalEntity, Model as PrincipalModel,
};
use super::PostgresIamRepository;

impl PostgresIamRepository {
    /// Load a Principal from DB and convert to domain model.
    pub(super) async fn load_principal_domain(&self, id: Uuid) -> Result<Option<Principal>, IamError> {
        let result = PrincipalEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(principal_model_to_domain(model))),
            None => Ok(None),
        }
    }

    /// Save a domain Principal to DB.
    pub(super) async fn save_principal_domain(&self, principal: &Principal) -> Result<(), IamError> {
        let model = principal_domain_to_model(principal);

        let exists = PrincipalEntity::find_by_id(principal.id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?
            .is_some();

        if exists {
            PrincipalEntity::update(PrincipalActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;
        } else {
            PrincipalEntity::insert(PrincipalActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;
        }
        Ok(())
    }

    /// Find a Principal by email using unique index.
    pub(super) async fn find_principal_by_email_domain(&self, email: &str) -> Result<Option<Principal>, IamError> {
        let result = PrincipalEntity::find()
            .filter(PrincipalColumn::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(principal_model_to_domain(model))),
            None => Ok(None),
        }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn principal_domain_to_model(p: &Principal) -> PrincipalModel {
    let roles_json = serde_json::to_string(&p.roles).unwrap_or_default();
    let permissions_json = serde_json::to_string(&p.permissions).unwrap_or_default();
    PrincipalModel {
        id: p.id,
        principal_type: p.principal_type.as_str().to_string(),
        email: p.email.clone(),
        password_hash: p.password_hash.clone(),
        mfa_enrolled: p.mfa_enrolled,
        mfa_method: p.mfa_method.as_ref().map(|m| m.as_str().to_string()),
        status: p.status.as_str().to_string(),
        roles: roles_json,
        permissions: permissions_json,
        operator_id: p.operator_id,
        failed_login_attempts: p.failed_login_attempts,
        locked_until: p.locked_until,
        created_at: p.created_at,
        last_login_at: p.last_login_at,
        updated_at: p.updated_at,
    }
}

fn principal_model_to_domain(m: PrincipalModel) -> Principal {
    let roles: Vec<String> = serde_json::from_str(&m.roles).unwrap_or_default();
    let permissions: Vec<String> = serde_json::from_str(&m.permissions).unwrap_or_default();
    Principal {
        id: m.id,
        principal_type: PrincipalType::parse_str(&m.principal_type).unwrap_or(PrincipalType::Human),
        email: m.email,
        password_hash: m.password_hash,
        mfa_enrolled: m.mfa_enrolled,
        mfa_method: m.mfa_method.as_deref().and_then(MfaMethod::parse_str),
        status: PrincipalStatus::parse_str(&m.status).unwrap_or(PrincipalStatus::Active),
        roles,
        permissions,
        operator_id: m.operator_id,
        failed_login_attempts: m.failed_login_attempts,
        locked_until: m.locked_until,
        created_at: m.created_at,
        last_login_at: m.last_login_at,
        updated_at: m.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_principal_domain_entity_roundtrip() {
        let now = Utc::now();
        let p = Principal {
            id: Uuid::now_v7(),
            principal_type: PrincipalType::Human,
            email: Some("test@example.com".into()),
            password_hash: Some(vec![1, 2, 3, 4]),
            mfa_enrolled: true,
            mfa_method: Some(MfaMethod::Totp),
            status: PrincipalStatus::Active,
            roles: vec!["operator".into()],
            permissions: vec!["payment:read".into()],
            operator_id: Some(Uuid::now_v7()),
            failed_login_attempts: 2,
            locked_until: None,
            created_at: now,
            last_login_at: Some(now),
            updated_at: now,
        };

        let model = principal_domain_to_model(&p);
        let roundtrip = principal_model_to_domain(model);

        assert_eq!(roundtrip.id, p.id);
        assert_eq!(roundtrip.email, p.email);
        assert_eq!(roundtrip.principal_type.as_str(), p.principal_type.as_str());
        assert_eq!(roundtrip.status.as_str(), p.status.as_str());
        assert_eq!(roundtrip.mfa_method.map(|m| m.as_str().to_string()), Some("totp".into()));
        assert_eq!(roundtrip.failed_login_attempts, 2);
        assert_eq!(roundtrip.roles, vec!["operator".to_string()]);
        assert_eq!(roundtrip.permissions, vec!["payment:read".to_string()]);
        assert!(roundtrip.operator_id.is_some());
    }
}
