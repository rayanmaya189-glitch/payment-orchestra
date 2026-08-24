//! PostgreSQL-backed Invoice repository using SeaORM CRUD.
//!
//! Converts between the domain Invoice model (with InvoiceStatus enum, JSONB line_items)
//! and the flat SeaORM entity model (invoices table with JSONB columns).

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::*;
use crate::entities::{
    ActiveModel as InvoiceActiveModel, Column as InvoiceColumn,
    Entity as InvoiceEntity, Model as InvoiceModel,
};

/// PostgreSQL-backed repository implementing InvoiceRepository.
#[derive(Clone)]
pub struct PostgresInvoiceRepository {
    pub db: sea_orm::DatabaseConnection,
}

impl PostgresInvoiceRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn invoice_domain_to_model(inv: &Invoice) -> Result<InvoiceModel, InvoiceError> {
    let line_items_json = serde_json::to_value(&inv.line_items)
        .map_err(|e| InvoiceError::DatabaseError(format!("Serialize line_items: {e}")))?;
    let payment_intent_ids_json = serde_json::to_value(&inv.payment_intent_ids)
        .map_err(|e| InvoiceError::DatabaseError(format!("Serialize payment_intent_ids: {e}")))?;

    Ok(InvoiceModel {
        invoice_id: inv.invoice_id,
        operator_id: inv.operator_id,
        order_reference: inv.order_reference.clone(),
        status: inv.status.to_string(),
        line_items: line_items_json,
        total_amount_minor: inv.total_amount_minor,
        paid_amount_minor: inv.paid_amount_minor,
        currency: inv.currency.clone(),
        due_date: inv.due_date,
        recipient_email: inv.recipient_email.clone(),
        payment_intent_ids: payment_intent_ids_json,
        created_at: inv.created_at,
        updated_at: inv.updated_at,
    })
}

fn invoice_model_to_domain(m: InvoiceModel) -> Result<Invoice, InvoiceError> {
    let line_items: Vec<InvoiceLineItem> = serde_json::from_value(m.line_items)
        .map_err(|e| InvoiceError::DatabaseError(format!("Deserialize line_items: {e}")))?;
    let payment_intent_ids: Vec<Uuid> = serde_json::from_value(m.payment_intent_ids)
        .map_err(|e| InvoiceError::DatabaseError(format!("Deserialize payment_intent_ids: {e}")))?;
    let status: InvoiceStatus = m.status
        .parse()
        .map_err(|e: String| InvoiceError::DatabaseError(format!("Parse status: {e}")))?;

    Ok(Invoice {
        invoice_id: m.invoice_id,
        operator_id: m.operator_id,
        order_reference: m.order_reference,
        status,
        line_items,
        total_amount_minor: m.total_amount_minor,
        paid_amount_minor: m.paid_amount_minor,
        currency: m.currency,
        due_date: m.due_date,
        recipient_email: m.recipient_email,
        payment_intent_ids,
        created_at: m.created_at,
        updated_at: m.updated_at,
        pending_events: Vec::new(),
    })
}

// ─── InvoiceRepository Trait Implementation ──────────────────────────────────

use crate::repository::InvoiceRepository;

#[async_trait]
impl InvoiceRepository for PostgresInvoiceRepository {
    async fn load_invoice(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
        let result = InvoiceEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(invoice_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save_invoice(&self, invoice: &mut Invoice) -> Result<(), InvoiceError> {
        let model = invoice_domain_to_model(invoice)?;

        let exists = InvoiceEntity::find_by_id(invoice.invoice_id)
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?
            .is_some();

        if exists {
            InvoiceEntity::update(InvoiceActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;
        } else {
            InvoiceEntity::insert(InvoiceActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn find_by_order_reference(
        &self,
        operator_id: Uuid,
        order_ref: &str,
    ) -> Result<Option<Invoice>, InvoiceError> {
        let result = InvoiceEntity::find()
            .filter(InvoiceColumn::OperatorId.eq(operator_id))
            .filter(InvoiceColumn::OrderReference.eq(order_ref))
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(invoice_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<Invoice>, InvoiceError> {
        // Use PostgreSQL JSONB @> containment operator for efficient lookup.
        let payment_intent_json = serde_json::json!([payment_intent_id.to_string()]);
        let result = InvoiceEntity::find()
            .filter(
                sea_orm::sea_query::Expr::cust(
                    &format!("payment_intent_ids @> '{}'::jsonb", payment_intent_json)
                )
            )
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(invoice_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn find_overdue(&self, _operator_id: Uuid) -> Result<Vec<Invoice>, InvoiceError> {
        let now = Utc::now();
        let results = InvoiceEntity::find()
            .filter(InvoiceColumn::DueDate.lt(now))
            .filter(InvoiceColumn::Status.ne("paid"))
            .filter(InvoiceColumn::Status.ne("cancelled"))
            .all(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(invoice_model_to_domain)
            .collect()
    }

    async fn list_invoices(
        &self,
        _operator_id: Uuid,
        status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        let mut query = InvoiceEntity::find();

        if let Some(ref status) = status_filter {
            query = query.filter(InvoiceColumn::Status.eq(status.to_string()));
        }

        let results = query
            .order_by_desc(InvoiceColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(format!("Database error: {e}")))?;

        results
            .into_iter()
            .map(invoice_model_to_domain)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::InvoiceEvent;

    #[test]
    fn test_invoice_domain_entity_roundtrip() {
        let now = Utc::now();
        let invoice_id = Uuid::now_v7();
        let inv = Invoice {
            invoice_id,
            operator_id: Uuid::now_v7(),
            order_reference: "ORD-001".into(),
            status: InvoiceStatus::Draft,
            line_items: vec![InvoiceLineItem {
                description: "Item A".into(),
                amount_minor: 1000,
                quantity: 2,
                unit_price_minor: 500,
            }],
            total_amount_minor: 1000,
            paid_amount_minor: 0,
            currency: "USD".into(),
            due_date: now + chrono::Duration::days(30),
            recipient_email: Some("test@example.com".into()),
            payment_intent_ids: vec![],
            created_at: now,
            updated_at: now,
            pending_events: Vec::new(),
        };

        let model = invoice_domain_to_model(&inv).unwrap();
        let roundtrip = invoice_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.invoice_id, invoice_id);
        assert_eq!(roundtrip.order_reference, "ORD-001");
        assert_eq!(roundtrip.status, InvoiceStatus::Draft);
        assert_eq!(roundtrip.line_items.len(), 1);
        assert_eq!(roundtrip.line_items[0].description, "Item A");
        assert_eq!(roundtrip.line_items[0].amount_minor, 1000);
        assert_eq!(roundtrip.total_amount_minor, 1000);
        assert_eq!(roundtrip.paid_amount_minor, 0);
        assert!(roundtrip.pending_events.is_empty());

        // Test serialization roundtrip via serde_json
        let json = serde_json::to_value(&roundtrip).unwrap();
        let deserialized: Invoice = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.invoice_id, invoice_id);
        assert_eq!(deserialized.status, InvoiceStatus::Draft);
    }
}
