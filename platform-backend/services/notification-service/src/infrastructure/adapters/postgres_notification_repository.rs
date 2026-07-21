use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

use crate::domain::aggregates::Notification;
use crate::domain::rules::NotificationRepository;
use crate::domain::value_objects::{NotificationStatus, NotificationType};
use crate::infrastructure::entities::notification_entity;
use platform_error::PlatformError;

pub struct PostgresNotificationRepository {
    db: DatabaseConnection,
}

impl PostgresNotificationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NotificationRepository for PostgresNotificationRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, PlatformError> {
        let m = notification_entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB find_by_id: {e}")))?;
        Ok(m.map(|m| m.into()))
    }

    async fn save(&self, n: &Notification) -> Result<(), PlatformError> {
        let existing = notification_entity::Entity::find_by_id(n.notification_id)
            .one(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB find_by_id: {e}")))?;

        if let Some(model) = existing {
            let mut a = notification_entity::ActiveModel::from(model);
            a.status = Set(n.status.as_str().to_string());
            a.provider_message_id = Set(n.provider_message_id.clone());
            a.retry_count = Set(n.retry_count);
            a.sent_at = Set(n.sent_at.map(|dt| dt.into()));
            a.delivered_at = Set(n.delivered_at.map(|dt| dt.into()));
            a.failed_at = Set(n.failed_at.map(|dt| dt.into()));
            a.error_message = Set(n.error_message.clone());
            a.updated_at = Set(n.updated_at.into());
            a.update(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB update: {e}")))?;
        } else {
            let a = notification_entity::ActiveModel {
                notification_id: Set(n.notification_id),
                operator_id: Set(n.operator_id),
                notification_type: Set(n.notification_type.as_str().to_string()),
                status: Set(n.status.as_str().to_string()),
                recipient: Set(n.recipient.clone()),
                subject: Set(n.subject.clone()),
                body: Set(n.body.clone()),
                template_id: Set(n.template_id.clone()),
                template_data: Set(
                    n.template_data
                        .as_ref()
                        .and_then(|v| serde_json::to_value(v).ok()),
                ),
                provider_message_id: Set(n.provider_message_id.clone()),
                retry_count: Set(n.retry_count),
                sent_at: Set(n.sent_at.map(|dt| dt.into())),
                delivered_at: Set(n.delivered_at.map(|dt| dt.into())),
                failed_at: Set(n.failed_at.map(|dt| dt.into())),
                error_message: Set(n.error_message.clone()),
                created_at: Set(n.created_at.into()),
                updated_at: Set(n.updated_at.into()),
            };
            a.insert(&self.db)
                .await
                .map_err(|e| PlatformError::Internal(format!("DB insert: {e}")))?;
        }
        Ok(())
    }

    async fn find_retryable(&self) -> Result<Vec<Notification>, PlatformError> {
        let ms = notification_entity::Entity::find()
            .filter(notification_entity::Column::Status.eq("failed"))
            .filter(notification_entity::Column::RetryCount.lt(3))
            .order_by_asc(notification_entity::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB find_retryable: {e}")))?;
        Ok(ms.into_iter().map(|m| m.into()).collect())
    }

    async fn find_by_operator(
        &self,
        operator_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Notification>, PlatformError> {
        let ms = notification_entity::Entity::find()
            .filter(notification_entity::Column::OperatorId.eq(operator_id))
            .order_by_desc(notification_entity::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB find_by_operator: {e}")))?;
        Ok(ms.into_iter().map(|m| m.into()).collect())
    }

    async fn count_by_operator(&self, operator_id: Uuid) -> Result<i64, PlatformError> {
        use sea_orm::PaginatorTrait;
        let count = notification_entity::Entity::find()
            .filter(notification_entity::Column::OperatorId.eq(operator_id))
            .count(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB count_by_operator: {e}")))?;
        Ok(count as i64)
    }

    async fn find_by_status(
        &self,
        status: NotificationStatus,
        limit: u64,
    ) -> Result<Vec<Notification>, PlatformError> {
        let ms = notification_entity::Entity::find()
            .filter(notification_entity::Column::Status.eq(status.as_str()))
            .order_by_desc(notification_entity::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| PlatformError::Internal(format!("DB find_by_status: {e}")))?;
        Ok(ms.into_iter().map(|m| m.into()).collect())
    }
}

impl From<notification_entity::Model> for Notification {
    fn from(m: notification_entity::Model) -> Self {
        Notification {
            notification_id: m.notification_id,
            operator_id: m.operator_id,
            notification_type: NotificationType::from_str(&m.notification_type),
            status: NotificationStatus::from_str(&m.status),
            recipient: m.recipient,
            subject: m.subject,
            body: m.body,
            template_id: m.template_id,
            template_data: m
                .template_data
                .and_then(|v| serde_json::from_value(v.into()).ok()),
            provider_message_id: m.provider_message_id,
            retry_count: m.retry_count,
            max_retries: 3,
            sent_at: m.sent_at.map(|dt| dt.into()),
            delivered_at: m.delivered_at.map(|dt| dt.into()),
            failed_at: m.failed_at.map(|dt| dt.into()),
            error_message: m.error_message,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}
