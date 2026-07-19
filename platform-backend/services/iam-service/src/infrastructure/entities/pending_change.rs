use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::{PendingChange, PendingChangeStatus};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pending_changes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub change_id: Uuid,
    pub change_type: String,
    pub maker_id: Uuid,
    pub checker_id: Option<Uuid>,
    pub payload: Vec<u8>,
    pub status: String,
    pub maker_note: Option<String>,
    pub checker_note: Option<String>,
    pub requested_at: DateTimeWithTimeZone,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    pub expires_at: DateTimeWithTimeZone,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> PendingChange {
        PendingChange {
            change_id: self.change_id,
            change_type: self.change_type.clone(),
            maker_id: self.maker_id,
            checker_id: self.checker_id,
            payload: self.payload.clone(),
            status: PendingChangeStatus::from_str(&self.status),
            maker_note: self.maker_note.clone(),
            checker_note: self.checker_note.clone(),
            requested_at: self.requested_at.into(),
            reviewed_at: self.reviewed_at.map(|dt| dt.into()),
            expires_at: self.expires_at.into(),
        }
    }
}

impl From<PendingChange> for ActiveModel {
    fn from(c: PendingChange) -> Self {
        Self {
            change_id: sea_orm::Set(c.change_id),
            change_type: sea_orm::Set(c.change_type),
            maker_id: sea_orm::Set(c.maker_id),
            checker_id: sea_orm::Set(c.checker_id),
            payload: sea_orm::Set(c.payload),
            status: sea_orm::Set(c.status.as_str().to_string()),
            maker_note: sea_orm::Set(c.maker_note),
            checker_note: sea_orm::Set(c.checker_note),
            requested_at: sea_orm::Set(c.requested_at.into()),
            reviewed_at: sea_orm::Set(c.reviewed_at.map(|dt| dt.into())),
            expires_at: sea_orm::Set(c.expires_at.into()),
            created_at: sea_orm::Set(chrono::Utc::now().into()),
        }
    }
}

