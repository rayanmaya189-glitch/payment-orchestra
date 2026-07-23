//! PostgreSQL-backed NotificationRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::NotificationRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as NotificationActiveModel,
    Column as NotificationColumn,
    Entity as NotificationEntity,
    Model as NotificationModel,
};

pub struct PostgresNotificationRepository {
    pub db: DatabaseConnection,
}

impl PostgresNotificationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NotificationRepository for PostgresNotificationRepository {
    async fn load(&self, id: Uuid) -> Result<Option<NotificationRequest>, NotificationError> {
        let result = NotificationEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| NotificationError::NotFound(id))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, notification: &NotificationRequest) -> Result<(), NotificationError> {
        let model = domain_to_model(notification);
        let exists = NotificationEntity::find_by_id(notification.notification_id)
            .one(&self.db)
            .await
            .map_err(|e| NotificationError::NotFound(notification.notification_id))?
            .is_some();

        if exists {
            NotificationEntity::update(NotificationActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| NotificationError::NotFound(notification.notification_id))?;
        } else {
            NotificationEntity::insert(NotificationActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| NotificationError::NotFound(notification.notification_id))?;
        }
        Ok(())
    }

    async fn find_by_operator(&self, operator_id: Uuid) -> Result<Vec<NotificationRequest>, NotificationError> {
        let models = NotificationEntity::find()
            .filter(NotificationColumn::OperatorId.eq(operator_id))
            .all(&self.db)
            .await
            .map_err(|e| NotificationError::NotFound(operator_id))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_pending(&self, max_count: u32) -> Result<Vec<NotificationRequest>, NotificationError> {
        let models = NotificationEntity::find()
            .filter(NotificationColumn::Status.eq("pending"))
            .limit(max_count as u64)
            .all(&self.db)
            .await
            .map_err(|e| NotificationError::NotFound(Uuid::default()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_dead_letter(&self, max_count: u32) -> Result<Vec<NotificationRequest>, NotificationError> {
        let models = NotificationEntity::find()
            .filter(NotificationColumn::Status.eq("dead_letter"))
            .limit(max_count as u64)
            .all(&self.db)
            .await
            .map_err(|e| NotificationError::NotFound(Uuid::default()))?;
        models.into_iter().map(model_to_domain).collect()
    }
}

fn domain_to_model(n: &NotificationRequest) -> NotificationModel {
    NotificationModel {
        notification_id: n.notification_id,
        operator_id: n.operator_id,
        channel: n.channel.as_str().to_string(),
        recipient: n.recipient.clone(),
        template_id: n.template_id.clone(),
        payload_json: n.payload_json.clone(),
        subject: n.subject.clone(),
        status: n.status.as_str().to_string(),
        retry_count: n.retry_count,
        max_retries: n.max_retries,
        created_at: n.created_at,
        sent_at: n.sent_at,
        last_error: n.last_error.clone(),
    }
}

fn model_to_domain(m: NotificationModel) -> Result<NotificationRequest, NotificationError> {
    let channel = NotificationChannel::from_str(&m.channel)
        .ok_or_else(|| NotificationError::NotFound(m.notification_id))?;
    let status = DeliveryStatus::from_str(&m.status)
        .ok_or_else(|| NotificationError::NotFound(m.notification_id))?;

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
