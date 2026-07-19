use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "operator")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub legal_name: String,
    pub trade_license_no: String,
    pub country: String,
    pub status: String,
    pub subdomain: String,
    pub email: String,
    pub provisioned_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> crate::domain::aggregates::Operator {
        crate::domain::aggregates::Operator {
            id: self.id,
            legal_name: self.legal_name.clone(),
            trade_license_no: crate::domain::value_objects::TradeLicenseNo::new(&self.trade_license_no)
                .expect("DB contains invalid trade_license_no"),
            country: self.country.clone(),
            status: crate::domain::value_objects::OperatorStatus::from_str(&self.status)
                .unwrap_or(crate::domain::value_objects::OperatorStatus::Pending),
            subdomain: self.subdomain.clone(),
            email: self.email.clone(),
            provisioned_at: self.provisioned_at.map(|dt| dt.into()),
            created_at: self.created_at.into(),
        }
    }
}

impl From<crate::domain::aggregates::Operator> for ActiveModel {
    fn from(op: crate::domain::aggregates::Operator) -> Self {
        Self {
            id: sea_orm::Set(op.id),
            legal_name: sea_orm::Set(op.legal_name),
            trade_license_no: sea_orm::Set(op.trade_license_no.to_string()),
            country: sea_orm::Set(op.country),
            status: sea_orm::Set(op.status.to_string()),
            subdomain: sea_orm::Set(op.subdomain),
            email: sea_orm::Set(op.email),
            provisioned_at: sea_orm::Set(op.provisioned_at.map(|dt| dt.into())),
            created_at: sea_orm::Set(op.created_at.into()),
        }
    }
}
