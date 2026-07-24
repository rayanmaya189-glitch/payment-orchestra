//! PostgreSQL-backed InvoiceRepository using SeaORM + platform-db entities.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::InvoiceRepository;
use crate::domain::*;
use crate::entities::{
    Entity as InvoiceEntity,
    ActiveModel as InvoiceActiveModel,
    Model as InvoiceModel,
    Column as InvoiceColumn,
};

/// SeaORM-backed invoice repository.
pub struct PostgresInvoiceRepository {
    pub db: DatabaseConnection,
}

impl PostgresInvoiceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl InvoiceRepository for PostgresInvoiceRepository {
    async fn load_invoice(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
        let result = InvoiceEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_invoice(&self, invoice: &mut Invoice) -> Result<(), InvoiceError> {
        let model = domain_to_model(invoice)?;
        let exists = InvoiceEntity::find_by_id(invoice.invoice_id)
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            InvoiceEntity::update(InvoiceActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;
        } else {
            InvoiceEntity::insert(InvoiceActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn find_by_order_reference(
        &self,
        operator_id: Uuid,
        order_ref: &str,
    ) -> Result<Option<Invoice>, InvoiceError> {
        let result = InvoiceEntity::find()
            .filter(
                sea_orm::Condition::all()
                    .add(InvoiceColumn::OperatorId.eq(operator_id))
                    .add(InvoiceColumn::OrderReference.eq(order_ref)),
            )
            .one(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<Invoice>, InvoiceError> {
        // Payment intent IDs are stored as JSONB array — fetch all and filter in Rust
        let all = InvoiceEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;

        for model in all {
            let pi_ids: Vec<Uuid> = serde_json::from_value(model.payment_intent_ids.clone())
                .map_err(|e| InvoiceError::General(format!("Invalid payment_intent_ids JSON: {}", e)))?;
            if pi_ids.contains(&payment_intent_id) {
                return Ok(Some(model_to_domain(model)?));
            }
        }
        Ok(None)
    }

    async fn find_overdue(&self, operator_id: Uuid) -> Result<Vec<Invoice>, InvoiceError> {
        let now = Utc::now();
        let models = InvoiceEntity::find()
            .filter(InvoiceColumn::OperatorId.eq(operator_id))
            .filter(InvoiceColumn::Status.eq("sent"))
            .filter(InvoiceColumn::DueDate.lt(now))
            .all(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn list_invoices(
        &self,
        operator_id: Uuid,
        status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>, InvoiceError> {
        let mut query = InvoiceEntity::find()
            .filter(InvoiceColumn::OperatorId.eq(operator_id));

        if let Some(ref filter) = status_filter {
            query = query.filter(InvoiceColumn::Status.eq(filter.to_string()));
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| InvoiceError::DatabaseError(e.to_string()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

// ─── Domain ↔ Model conversion ───────────────────────────────────────────────

fn domain_to_model(inv: &Invoice) -> Result<InvoiceModel, InvoiceError> {
    Ok(InvoiceModel {
        invoice_id: inv.invoice_id,
        operator_id: inv.operator_id,
        order_reference: inv.order_reference.clone(),
        status: inv.status.to_string(),
        line_items: serde_json::to_value(&inv.line_items)
            .map_err(|e| InvoiceError::General(format!("Serialize line_items: {}", e)))?,
        total_amount_minor: inv.total_amount_minor,
        paid_amount_minor: inv.paid_amount_minor,
        currency: inv.currency.clone(),
        due_date: inv.due_date,
        recipient_email: inv.recipient_email.clone(),
        payment_intent_ids: serde_json::to_value(&inv.payment_intent_ids)
            .map_err(|e| InvoiceError::General(format!("Serialize payment_intent_ids: {}", e)))?,
        created_at: inv.created_at,
        updated_at: inv.updated_at,
    })
}

fn model_to_domain(m: InvoiceModel) -> Result<Invoice, InvoiceError> {
    let status: InvoiceStatus = m.status.parse()
        .map_err(|e: String| InvoiceError::General(e))?;

    let line_items: Vec<InvoiceLineItem> = serde_json::from_value(m.line_items)
        .map_err(|e| InvoiceError::General(format!("Deserialize line_items: {}", e)))?;

    let payment_intent_ids: Vec<Uuid> = serde_json::from_value(m.payment_intent_ids)
        .map_err(|e| InvoiceError::General(format!("Deserialize payment_intent_ids: {}", e)))?;

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
