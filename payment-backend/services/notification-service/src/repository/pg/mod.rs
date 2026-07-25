//! PostgreSQL-backed NotificationRepository using SeaORM CRUD.
//! WebhookRepository uses in-memory storage (no webhooks table entity yet).

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::entities::notification::{
    ActiveModel as NotificationActiveModel, Column as NotificationColumn,
    Entity as NotificationEntity, Model as NotificationModel,
};

/// Combined repository implementing both NotificationRepository and WebhookRepository.
#[derive(Clone)]
pub struct PostgresNotificationRepository {
    pub db: sea_orm::DatabaseConnection,
    /// In-memory webhook storage (no webhooks table entity yet)
    webhooks: Arc<RwLock<HashMap<Uuid, Webhook>>>,
}

impl PostgresNotificationRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self {
            db,
            webhooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn domain_to_model(n: &NotificationRequest) -> NotificationModel {
    NotificationModel {
        notification_id: n.notification_id,
        operator_id: n.operator_id,
        channel: n.channel.to_string(),
        recipient: n.recipient.clone(),
        template_id: n.template_id.clone(),
        payload_json: n.payload_json.clone(),
        subject: n.subject.clone(),
        status: n.status.to_string(),
        retry_count: n.retry_count,
        max_retries: n.max_retries,
        created_at: n.created_at,
        sent_at: n.sent_at,
        last_error: n.last_error.clone(),
    }
}

fn model_to_domain(m: NotificationModel) -> Result<NotificationRequest, NotificationError> {
    let channel: NotificationChannel = match m.channel.as_str() {
        "email" => NotificationChannel::Email,
        "sms" => NotificationChannel::Sms,
        "webhook" => NotificationChannel::Webhook,
        other => return Err(NotificationError::TemplateMissing(format!("Invalid channel: {other}"))),
    };
    let status: DeliveryStatus = m.status
        .parse()
        .map_err(|e: String| NotificationError::TemplateMissing(e))?;

    Ok(NotificationRequest {
        notification_id: m.notification_id,
        operator_id: m.operator_id,
        channel,
        recipient: m.recipient,
        template_id: m.template_id,
        payload_json: m.payload_json,
        subject: m.subject,
        status,
        retry_count: m.retry_count,
        max_retries: m.max_retries,
        created_at: m.created_at,
        sent_at: m.sent_at,
        last_error: m.last_error,
    })
}

// ─── NotificationRepository Trait Implementation ─────────────────────────────

use crate::repository::NotificationRepository;

#[async_trait]
impl NotificationRepository for PostgresNotificationRepository {
    async fn load(&self, id: Uuid) -> Result<Option<NotificationRequest>, NotificationError> {
        let result = NotificationEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?;

        match result {
            Some(model) => Ok(Some(model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, request: &NotificationRequest) -> Result<(), NotificationError> {
        let model = domain_to_model(request);

        let exists = NotificationEntity::find_by_id(request.notification_id)
            .one(&self.db)
            .await
            .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?
            .is_some();

        if exists {
            NotificationEntity::update(NotificationActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?;
        } else {
            NotificationEntity::insert(NotificationActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?;
        }

        Ok(())
    }

    async fn find_pending(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let results = NotificationEntity::find()
            .filter(NotificationColumn::Status.eq("queued"))
            .all(&self.db)
            .await
            .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn find_dead_letter(&self) -> Result<Vec<NotificationRequest>, NotificationError> {
        let results = NotificationEntity::find()
            .filter(NotificationColumn::Status.eq("dead_letter"))
            .all(&self.db)
            .await
            .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?;

        results.into_iter().map(model_to_domain).collect()
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        let results = NotificationEntity::find()
            .filter(NotificationColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| NotificationError::TemplateMissing(format!("Database error: {e}")))?;

        results.into_iter().map(model_to_domain).collect()
    }
}

// ─── WebhookRepository Trait Implementation (in-memory) ─────────────────────

use crate::repository::WebhookRepository;

#[async_trait]
impl WebhookRepository for PostgresNotificationRepository {
    async fn save(&self, webhook: &Webhook) -> Result<(), NotificationError> {
        let mut wh = self.webhooks.write().await;
        wh.insert(webhook.webhook_id, webhook.clone());
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Webhook>, NotificationError> {
        let wh = self.webhooks.read().await;
        Ok(wh.get(&id).cloned())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<Webhook>, NotificationError> {
        let wh = self.webhooks.read().await;
        Ok(wh.values().filter(|w| w.operator_id == operator_id).cloned().collect())
    }

    async fn delete(&self, id: Uuid) -> Result<(), NotificationError> {
        let mut wh = self.webhooks.write().await;
        wh.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_domain_entity_roundtrip() {
        let n = NotificationRequest::new(
            Uuid::now_v7(),
            NotificationChannel::Email,
            "test@example.com".into(),
            "direct_email".into(),
            r#"{"body":"Hello"}"#.into(),
            Some("Test Subject".into()),
        );

        let model = domain_to_model(&n);
        let roundtrip = model_to_domain(model).unwrap();

        assert_eq!(roundtrip.notification_id, n.notification_id);
        assert_eq!(roundtrip.channel, NotificationChannel::Email);
        assert_eq!(roundtrip.recipient, "test@example.com");
        assert_eq!(roundtrip.status, DeliveryStatus::Queued);
        assert_eq!(roundtrip.retry_count, 0);
        assert_eq!(roundtrip.max_retries, 3);
        assert!(roundtrip.subject.is_some());
    }

    #[test]
    fn test_delivery_status_serialization() {
        assert_eq!(DeliveryStatus::Queued.to_string(), "queued");
        assert_eq!(DeliveryStatus::Sent.to_string(), "sent");
        assert_eq!(DeliveryStatus::Failed.to_string(), "failed");
        assert_eq!(DeliveryStatus::DeadLetter.to_string(), "dead_letter");

        assert_eq!("queued".parse::<DeliveryStatus>().unwrap(), DeliveryStatus::Queued);
        assert_eq!("sent".parse::<DeliveryStatus>().unwrap(), DeliveryStatus::Sent);
    }
}
