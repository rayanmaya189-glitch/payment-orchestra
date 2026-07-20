use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::aggregates::Invoice;
use crate::domain::value_objects::InvoiceStatus;
use shared_types::{CurrencyCode, Money};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "invoice")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub invoice_id: Uuid,
    pub operator_id: Uuid,
    pub order_reference: String,
    pub status: String,
    pub total_amount_minor_units: i64,
    pub paid_amount_minor_units: i64,
    pub currency: String,
    pub line_items_json: String,
    pub due_date: DateTimeWithTimeZone,
    pub recipient_email: Option<String>,
    pub payment_intent_ids: String, // JSON array
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_domain(&self) -> Invoice {
        let line_items = serde_json::from_str(&self.line_items_json).unwrap_or_default();
        let payment_intent_ids = serde_json::from_str(&self.payment_intent_ids).unwrap_or_default();
        let currency = CurrencyCode::new(&self.currency).unwrap();

        Invoice {
            invoice_id: self.invoice_id,
            operator_id: self.operator_id,
            order_reference: self.order_reference.clone(),
            status: InvoiceStatus::from_str(&self.status)
                .expect("DB contains invalid invoice status"),
            total_amount: Money {
                amount_minor_units: self.total_amount_minor_units,
                currency: currency.clone(),
            },
            paid_amount: Money {
                amount_minor_units: self.paid_amount_minor_units,
                currency,
            },
            line_items,
            due_date: self.due_date.into(),
            recipient_email: self.recipient_email.clone(),
            payment_intent_ids,
            created_at: self.created_at.into(),
            updated_at: self.updated_at.into(),
        }
    }
}

impl From<Invoice> for ActiveModel {
    fn from(i: Invoice) -> Self {
        Self {
            invoice_id: sea_orm::Set(i.invoice_id),
            operator_id: sea_orm::Set(i.operator_id),
            order_reference: sea_orm::Set(i.order_reference),
            status: sea_orm::Set(i.status.as_str().to_string()),
            total_amount_minor_units: sea_orm::Set(i.total_amount.amount_minor_units),
            paid_amount_minor_units: sea_orm::Set(i.paid_amount.amount_minor_units),
            currency: sea_orm::Set(i.total_amount.currency.0),
            line_items_json: sea_orm::Set(serde_json::to_string(&i.line_items).unwrap()),
            due_date: sea_orm::Set(i.due_date.into()),
            recipient_email: sea_orm::Set(i.recipient_email),
            payment_intent_ids: sea_orm::Set(serde_json::to_string(&i.payment_intent_ids).unwrap()),
            created_at: sea_orm::Set(i.created_at.into()),
            updated_at: sea_orm::Set(i.updated_at.into()),
        }
    }
}
