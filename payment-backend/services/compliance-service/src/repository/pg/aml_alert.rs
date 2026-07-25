//! PostgreSQL-backed AmlAlert repository using SeaORM CRUD.
//!
//! Converts between the domain AmlAlert model (with AmlAlertType, AlertSeverity,
//! AlertStatus enums) and the flat SeaORM entity model (aml_alerts table).

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::{
    AlertSeverity, AlertStatus, AmlAlert, AmlAlertType, ComplianceError,
};
use crate::entities::aml_alert::{
    ActiveModel as AmlAlertActiveModel, Column as AmlAlertColumn,
    Entity as AmlAlertEntity, Model as AmlAlertModel,
};
use super::PostgresComplianceRepository;

impl PostgresComplianceRepository {
    /// Load AmlAlert from DB and convert to domain model.
    pub(super) async fn load_aml_alert_domain(&self, id: Uuid) -> Result<Option<AmlAlert>, ComplianceError> {
        let result = AmlAlertEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;

        match result {
            Some(model) => Ok(Some(aml_alert_model_to_domain(model)?)),
            None => Ok(None),
        }
    }

    /// Save domain AmlAlert to DB.
    pub(super) async fn save_aml_alert_domain(&self, alert: &AmlAlert) -> Result<(), ComplianceError> {
        let model = aml_alert_domain_to_model(alert)?;

        let exists = AmlAlertEntity::find_by_id(alert.alert_id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?
            .is_some();

        if exists {
            AmlAlertEntity::update(AmlAlertActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;
        } else {
            AmlAlertEntity::insert(AmlAlertActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;
        }
        Ok(())
    }
}

// ─── AML Query methods ──────────────────────────────────────────────────

impl PostgresComplianceRepository {
    /// List AML alerts for an operator with optional status filter.
    pub(super) async fn list_aml_alerts_domain(
        &self,
        operator_id: Uuid,
        status_filter: Option<&str>,
    ) -> Result<Vec<AmlAlert>, ComplianceError> {
        let mut query = AmlAlertEntity::find()
            .filter(AmlAlertColumn::OperatorId.eq(operator_id));

        if let Some(status) = status_filter {
            query = query.filter(AmlAlertColumn::Status.eq(status));
        }

        let models = query
            .order_by_desc(AmlAlertColumn::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;

        models.into_iter().map(aml_alert_model_to_domain).collect()
    }

    /// Find AML alerts by transaction ID.
    pub(super) async fn find_alerts_by_transaction_domain(
        &self,
        transaction_id: Uuid,
    ) -> Result<Vec<AmlAlert>, ComplianceError> {
        let models = AmlAlertEntity::find()
            .filter(AmlAlertColumn::TransactionId.eq(transaction_id))
            .all(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(format!("Database error: {}", e)))?;

        models.into_iter().map(aml_alert_model_to_domain).collect()
    }
}

// ─── Domain ←→ Entity Conversion ─────────────────────────────────────────────

fn aml_alert_domain_to_model(alert: &AmlAlert) -> Result<AmlAlertModel, ComplianceError> {
    Ok(AmlAlertModel {
        alert_id: alert.alert_id,
        operator_id: alert.operator_id,
        transaction_id: alert.transaction_id,
        alert_type: alert.alert_type.as_str().to_string(),
        severity: alert.severity.as_str().to_string(),
        rule_id: alert.rule_id.clone(),
        details: alert.details.clone(),
        status: alert.status.as_str().to_string(),
        reviewed_by: alert.reviewed_by,
        reviewed_at: alert.reviewed_at,
        created_at: alert.created_at,
    })
}

fn aml_alert_model_to_domain(m: AmlAlertModel) -> Result<AmlAlert, ComplianceError> {
    Ok(AmlAlert {
        alert_id: m.alert_id,
        operator_id: m.operator_id,
        transaction_id: m.transaction_id,
        alert_type: AmlAlertType::parse_str(&m.alert_type).unwrap_or(AmlAlertType::Velocity),
        severity: AlertSeverity::parse_str(&m.severity).unwrap_or(AlertSeverity::Low),
        rule_id: m.rule_id,
        details: m.details,
        status: AlertStatus::parse_str(&m.status).unwrap_or(AlertStatus::Open),
        reviewed_by: m.reviewed_by,
        reviewed_at: m.reviewed_at,
        created_at: m.created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_aml_alert_domain_entity_roundtrip() {
        let now = Utc::now();
        let alert = AmlAlert {
            alert_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            transaction_id: Uuid::now_v7(),
            alert_type: AmlAlertType::Velocity,
            severity: AlertSeverity::High,
            rule_id: "AML-R003".into(),
            details: "25 transactions in 60 min window".into(),
            status: AlertStatus::Open,
            reviewed_by: None,
            reviewed_at: None,
            created_at: now,
        };

        let model = aml_alert_domain_to_model(&alert).unwrap();
        let roundtrip = aml_alert_model_to_domain(model).unwrap();

        assert_eq!(roundtrip.alert_id, alert.alert_id);
        assert_eq!(roundtrip.alert_type.as_str(), alert.alert_type.as_str());
        assert_eq!(roundtrip.severity.as_str(), alert.severity.as_str());
        assert_eq!(roundtrip.status.as_str(), alert.status.as_str());
        assert_eq!(roundtrip.rule_id, alert.rule_id);
        assert_eq!(roundtrip.details, alert.details);
    }
}
