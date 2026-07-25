//! PostgreSQL-backed PendingChange operations using SeaORM CRUD.
//!
//! Converts between the domain PendingChange model (with ChangeStatus enum)
//! and the flat SeaORM entity model (pending_changes table).

use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::domain::{ChangeStatus, IamError, PendingChange};
use crate::entities::pending_change::{
    ActiveModel as PendingChangeActiveModel, Entity as PendingChangeEntity,
    Model as PendingChangeModel,
};
use super::PostgresIamRepository;

impl PostgresIamRepository {
    /// Load a PendingChange from DB and convert to domain model.
    pub(super) async fn load_change_domain(&self, id: Uuid) -> Result<Option<PendingChange>, IamError> {
        let result = PendingChangeEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(pending_change_model_to_domain(model))),
            None => Ok(None),
        }
    }

    /// Save a domain PendingChange to DB.
    pub(super) async fn save_change_domain(&self, change: &PendingChange) -> Result<(), IamError> {
        let model = pending_change_domain_to_model(change);

        let exists = PendingChangeEntity::find_by_id(change.change_id)
            .one(&self.db)
            .await
            .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?
            .is_some();

        if exists {
            PendingChangeEntity::update(PendingChangeActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;
        } else {
            PendingChangeEntity::insert(PendingChangeActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| IamError::InvalidRequest(format!("Database error: {}", e)))?;
        }
        Ok(())
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn pending_change_domain_to_model(c: &PendingChange) -> PendingChangeModel {
    PendingChangeModel {
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
    }
}

fn pending_change_model_to_domain(m: PendingChangeModel) -> PendingChange {
    PendingChange {
        change_id: m.change_id,
        change_type: m.change_type,
        maker_id: m.maker_id,
        checker_id: m.checker_id,
        payload: m.payload,
        status: ChangeStatus::parse_str(&m.status).unwrap_or(ChangeStatus::Pending),
        maker_note: m.maker_note,
        checker_note: m.checker_note,
        requested_at: m.requested_at,
        reviewed_at: m.reviewed_at,
        expires_at: m.expires_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_pending_change_domain_entity_roundtrip() {
        let now = Utc::now();
        let change = PendingChange {
            change_id: Uuid::now_v7(),
            change_type: "update_gateway_config".into(),
            maker_id: Uuid::now_v7(),
            checker_id: Some(Uuid::now_v7()),
            payload: vec![1, 2, 3],
            status: ChangeStatus::Approved,
            maker_note: Some("Update fee structure".into()),
            checker_note: Some("Approved".into()),
            requested_at: now,
            reviewed_at: Some(now),
            expires_at: now + chrono::Duration::hours(72),
        };

        let model = pending_change_domain_to_model(&change);
        let roundtrip = pending_change_model_to_domain(model);

        assert_eq!(roundtrip.change_id, change.change_id);
        assert_eq!(roundtrip.change_type, change.change_type);
        assert_eq!(roundtrip.status.as_str(), change.status.as_str());
        assert_eq!(roundtrip.maker_note, change.maker_note);
        assert_eq!(roundtrip.checker_id, change.checker_id);
    }
}
