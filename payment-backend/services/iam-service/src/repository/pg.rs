//! PostgreSQL-backed IamRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::IamRepository;
use crate::domain::*;
use crate::entities::{
    api_key::{
        ActiveModel as ApiKeyActiveModel,
        Column as ApiKeyColumn,
        Entity as ApiKeyEntity,
        Model as ApiKeyModel,
    },
    pending_change::{
        ActiveModel as PendingChangeActiveModel,
        Column as PendingChangeColumn,
        Entity as PendingChangeEntity,
        Model as PendingChangeModel,
    },
    principal::{
        ActiveModel as PrincipalActiveModel,
        Column as PrincipalColumn,
        Entity as PrincipalEntity,
        Model as PrincipalModel,
    },
};

/// SeaORM-backed IAM repository.
pub struct PostgresIamRepository {
    pub db: DatabaseConnection,
}

impl PostgresIamRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
#[allow(clippy::too_many_lines)]
impl IamRepository for PostgresIamRepository {
    // ── Principal operations ──────────────────────────────────────────────

    async fn load_principal(&self, id: Uuid) -> Result<Option<Principal>, IamError> {
        let result = PrincipalEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(principal_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_principal(&self, p: &Principal) -> Result<(), IamError> {
        let model = principal_domain_to_model(p)?;
        let exists = PrincipalEntity::find_by_id(p.id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?
            .is_some();

        if exists {
            PrincipalEntity::update(PrincipalActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        } else {
            PrincipalEntity::insert(PrincipalActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_principal_by_email(&self, email: &str) -> Result<Option<Principal>, IamError> {
        let result = PrincipalEntity::find()
            .filter(PrincipalColumn::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(principal_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    // ── ApiKey operations ─────────────────────────────────────────────────

    async fn load_api_key(&self, id: Uuid) -> Result<Option<ApiKey>, IamError> {
        let result = ApiKeyEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(api_key_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_api_key(&self, key: &ApiKey) -> Result<(), IamError> {
        let model = api_key_domain_to_model(key)?;
        let exists = ApiKeyEntity::find_by_id(key.api_key_id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?
            .is_some();

        if exists {
            ApiKeyEntity::update(ApiKeyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        } else {
            ApiKeyEntity::insert(ApiKeyActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        }
        Ok(())
    }

    async fn list_api_keys_for_principal(&self, principal_id: Uuid) -> Result<Vec<ApiKey>, IamError> {
        let models = ApiKeyEntity::find()
            .filter(ApiKeyColumn::PrincipalId.eq(principal_id))
            .all(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(api_key_model_to_domain).collect()
    }

    async fn find_api_key_by_name(&self, principal_id: Uuid, name: &str) -> Result<Option<ApiKey>, IamError> {
        let result = ApiKeyEntity::find()
            .filter(
                sea_orm::Condition::all()
                    .add(ApiKeyColumn::PrincipalId.eq(principal_id))
                    .add(ApiKeyColumn::Name.eq(name)),
            )
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(api_key_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    // ── PendingChange operations ──────────────────────────────────────────

    async fn load_change(&self, id: Uuid) -> Result<Option<PendingChange>, IamError> {
        let result = PendingChangeEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(pending_change_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_change(&self, change: &PendingChange) -> Result<(), IamError> {
        let model = pending_change_domain_to_model(change)?;
        let exists = PendingChangeEntity::find_by_id(change.change_id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(e.to_string()))?
            .is_some();

        if exists {
            PendingChangeEntity::update(PendingChangeActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        } else {
            PendingChangeEntity::insert(PendingChangeActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(e.to_string()))?;
        }
        Ok(())
    }
}

// ─── Domain ↔ Model conversion: Principal ────────────────────────────────────

fn principal_domain_to_model(p: &Principal) -> Result<PrincipalModel, IamError> {
    Ok(PrincipalModel {
        id: p.id,
        principal_type: p.principal_type.as_str().to_string(),
        email: p.email.clone(),
        password_hash: p.password_hash.clone(),
        mfa_enrolled: p.mfa_enrolled,
        mfa_method: p.mfa_method.as_ref().map(|m| m.as_str().to_string()),
        status: p.status.as_str().to_string(),
        failed_login_attempts: p.failed_login_attempts,
        locked_until: p.locked_until,
        created_at: p.created_at,
        last_login_at: p.last_login_at,
        updated_at: p.updated_at,
    })
}

fn principal_model_to_domain(m: PrincipalModel) -> Result<Principal, IamError> {
    let principal_type = match m.principal_type.as_str() {
        "human" => PrincipalType::Human,
        "api_key" => PrincipalType::ApiKey,
        "service" => PrincipalType::Service,
        _ => return Err(IamError::InvalidRequest(format!("Unknown principal_type: {}", m.principal_type))),
    };

    let status = match m.status.as_str() {
        "active" => PrincipalStatus::Active,
        "suspended" => PrincipalStatus::Suspended,
        "deleted" => PrincipalStatus::Deleted,
        _ => return Err(IamError::InvalidRequest(format!("Unknown status: {}", m.status))),
    };

    let mfa_method = m.mfa_method.as_ref().and_then(|s| match s.as_str() {
        "webauthn" => Some(MfaMethod::WebAuthn),
        "totp" => Some(MfaMethod::Totp),
        _ => None,
    });

    Ok(Principal {
        id: m.id,
        principal_type,
        email: m.email,
        password_hash: m.password_hash,
        mfa_enrolled: m.mfa_enrolled,
        mfa_method,
        status,
        failed_login_attempts: m.failed_login_attempts,
        locked_until: m.locked_until,
        created_at: m.created_at,
        last_login_at: m.last_login_at,
        updated_at: m.updated_at,
    })
}

// ─── Domain ↔ Model conversion: ApiKey ───────────────────────────────────────

fn api_key_domain_to_model(k: &ApiKey) -> Result<ApiKeyModel, IamError> {
    let scopes = serde_json::to_value(&k.scopes)
        .map_err(|e| IamError::InvalidRequest(format!("Serialize scopes: {}", e)))?;

    Ok(ApiKeyModel {
        api_key_id: k.api_key_id,
        principal_id: k.principal_id,
        name: k.name.clone(),
        key_hash: k.key_hash.clone(),
        scopes,
        status: k.status.as_str().to_string(),
        created_at: k.created_at,
        expires_at: k.expires_at,
        last_used_at: k.last_used_at,
    })
}

fn api_key_model_to_domain(m: ApiKeyModel) -> Result<ApiKey, IamError> {
    let scopes: Vec<String> = serde_json::from_value(m.scopes)
        .map_err(|e| IamError::InvalidRequest(format!("Deserialize scopes: {}", e)))?;

    let status = match m.status.as_str() {
        "active" => ApiKeyStatus::Active,
        "revoked" => ApiKeyStatus::Revoked,
        "expired" => ApiKeyStatus::Expired,
        _ => return Err(IamError::InvalidRequest(format!("Unknown api_key status: {}", m.status))),
    };

    Ok(ApiKey {
        api_key_id: m.api_key_id,
        principal_id: m.principal_id,
        name: m.name,
        key_hash: m.key_hash,
        scopes,
        status,
        created_at: m.created_at,
        expires_at: m.expires_at,
        last_used_at: m.last_used_at,
    })
}

// ─── Domain ↔ Model conversion: PendingChange ────────────────────────────────

fn pending_change_domain_to_model(c: &PendingChange) -> Result<PendingChangeModel, IamError> {
    Ok(PendingChangeModel {
        change_id: c.change_id,
        change_type: c.change_type.clone(),
        maker_id: c.maker_id,
        checker_id: c.checker_id,
        payload: c.payload.clone(),
        status: c.status.as_str().to_string(),
        maker_note: c.maker_note.clone(),
        checker_note: c.checker_note.clone(),
        requested_at: c.requested_at,
        reviewed_at: c.reviewed_at,
        expires_at: c.expires_at,
    })
}

fn pending_change_model_to_domain(m: PendingChangeModel) -> Result<PendingChange, IamError> {
    let status = match m.status.as_str() {
        "pending" => ChangeStatus::Pending,
        "approved" => ChangeStatus::Approved,
        "rejected" => ChangeStatus::Rejected,
        "expired" => ChangeStatus::Expired,
        _ => return Err(IamError::InvalidRequest(format!("Unknown change status: {}", m.status))),
    };

    Ok(PendingChange {
        change_id: m.change_id,
        change_type: m.change_type,
        maker_id: m.maker_id,
        checker_id: m.checker_id,
        payload: m.payload,
        status,
        maker_note: m.maker_note,
        checker_note: m.checker_note,
        requested_at: m.requested_at,
        reviewed_at: m.reviewed_at,
        expires_at: m.expires_at,
    })
}
