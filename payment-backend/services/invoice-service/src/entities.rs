//! SeaORM entity model for the invoice-service's `invoices` table.
//!
//! Nested complex types (Vec) are stored as JSONB columns.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// `invoices` table entity.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "invoices")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Uuid")]
    pub invoice_id: Uuid,
    #[sea_orm(column_type = "Uuid")]
    pub operator_id: Uuid,
    pub order_reference: String,
    pub status: String,
    /// JSONB: serialized Vec<InvoiceLineItem>
    pub line_items: Json,
    pub total_amount_minor: i64,
    pub paid_amount_minor: i64,
    pub currency: String,
    pub due_date: DateTimeUtc,
    pub recipient_email: Option<String>,
    /// JSONB: serialized Vec<Uuid>
    pub payment_intent_ids: Json,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
