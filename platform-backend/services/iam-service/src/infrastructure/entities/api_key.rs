use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::ApiKey;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub principal_id: Uuid,
    pub name: String,
    pub key_hash: Vec<u8>,
    pub scopes: Json,
    pub acquirer_link_ids: Option<Json>,
    pub expires_at: DateTimeWithTimeZone,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::principal_entity::Entity",
        from = "Column::PrincipalId",
        to = "super::principal_entity::Column::Id"
    )]
    Principal,
}

impl Related<super::principal_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Principal.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    #[allow(dead_code)]
    pub fn to_domain(&self) -> ApiKey {
        let scopes: Vec<String> = serde_json::from_value(self.scopes.clone())
            .unwrap_or_default();
        let acquirer_link_ids: Option<Vec<Uuid>> = self.acquirer_link_ids.as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        ApiKey {
            id: self.id,
            principal_id: self.principal_id,
            name: self.name.clone(),
            key_hash: self.key_hash.clone(),
            scopes,
            acquirer_link_ids,
            expires_at: self.expires_at.into(),
            revoked_at: self.revoked_at.map(|dt| dt.into()),
            created_at: self.created_at.into(),
        }
    }
}

impl From<ApiKey> for ActiveModel {
    fn from(k: ApiKey) -> Self {
        Self {
            id: sea_orm::Set(k.id),
            principal_id: sea_orm::Set(k.principal_id),
            name: sea_orm::Set(k.name),
            key_hash: sea_orm::Set(k.key_hash),
            scopes: sea_orm::Set(serde_json::to_value(&k.scopes).unwrap()),
            acquirer_link_ids: sea_orm::Set(k.acquirer_link_ids.map(|v| serde_json::to_value(&v).unwrap())),
            expires_at: sea_orm::Set(k.expires_at.into()),
            revoked_at: sea_orm::Set(k.revoked_at.map(|dt| dt.into())),
            created_at: sea_orm::Set(k.created_at.into()),
        }
    }
}
