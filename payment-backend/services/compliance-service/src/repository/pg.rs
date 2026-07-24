//! PostgreSQL-backed ComplianceRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::ComplianceRepository;
use crate::domain::*;
use crate::entities::{
    aml_alert::{
        ActiveModel as AmlAlertActiveModel,
        Column as AmlAlertColumn,
        Entity as AmlAlertEntity,
        Model as AmlAlertModel,
    },
    kyb_case::{
        ActiveModel as KybCaseActiveModel,
        Column as KybCaseColumn,
        Entity as KybCaseEntity,
        Model as KybCaseModel,
    },
};

/// SeaORM-backed compliance repository.
pub struct PostgresComplianceRepository {
    pub db: DatabaseConnection,
}

impl PostgresComplianceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ComplianceRepository for PostgresComplianceRepository {
    // ── KYB operations ─────────────────────────────────────────────────────

    async fn load_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        let result = KybCaseEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(kyb_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_kyb_case(&self, kase: &KybCase) -> Result<(), ComplianceError> {
        let model = kyb_domain_to_model(kase)?;
        let exists = KybCaseEntity::find_by_id(kase.kyb_case_id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?
            .is_some();

        if exists {
            KybCaseEntity::update(KybCaseActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        } else {
            KybCaseEntity::insert(KybCaseActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        }
        Ok(())
    }

    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError> {
        let models = KybCaseEntity::find()
            .filter(KybCaseColumn::Status.is_in(vec!["submitted", "under_review"]))
            .all(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(kyb_model_to_domain).collect()
    }

    async fn find_kyb_by_operator(&self, operator_id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        let result = KybCaseEntity::find()
            .filter(KybCaseColumn::OperatorId.eq(operator_id))
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(kyb_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    // ── AML operations ────────────────────────────────────────────────────

    async fn load_aml_alert(&self, id: Uuid) -> Result<Option<AmlAlert>, ComplianceError> {
        let result = AmlAlertEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(aml_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_aml_alert(&self, alert: &AmlAlert) -> Result<(), ComplianceError> {
        let model = aml_domain_to_model(alert)?;
        let exists = AmlAlertEntity::find_by_id(alert.alert_id)
            .one(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?
            .is_some();

        if exists {
            AmlAlertEntity::update(AmlAlertActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        } else {
            AmlAlertEntity::insert(AmlAlertActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        }
        Ok(())
    }

    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError> {
        let mut query = AmlAlertEntity::find()
            .filter(AmlAlertColumn::OperatorId.eq(operator_id));

        if let Some(status) = status_filter {
            query = query.filter(AmlAlertColumn::Status.eq(status));
        }

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(aml_model_to_domain).collect()
    }

    async fn find_alerts_by_transaction(&self, transaction_id: Uuid) -> Result<Vec<AmlAlert>, ComplianceError> {
        let models = AmlAlertEntity::find()
            .filter(AmlAlertColumn::TransactionId.eq(transaction_id))
            .all(&self.db)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;
        models.into_iter().map(aml_model_to_domain).collect()
    }

    // ── Transaction history (queried via platform-db connection) ──────────

    async fn get_recent_transactions(&self, operator_id: Uuid, window_minutes: u32) -> Result<Vec<RecentTransaction>, ComplianceError> {
        // Query recent transactions from the payment_intents table via raw SQL
        let window = Utc::now() - chrono::Duration::minutes(window_minutes as i64);
        let results = sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            SELECT payment_intent_id AS transaction_id, amount_minor_units, created_at
            FROM payment_intents
            WHERE operator_id = $1 AND created_at >= $2
            ORDER BY created_at DESC
            "#,
            vec![operator_id.into(), window.into()],
        );

        let rows = self.db
            .query_all(results)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;

        let txns = rows.into_iter().filter_map(|row| {
            let transaction_id: Uuid = row.try_get::<Uuid>("", "transaction_id").ok()?;
            let amount_minor_units: i64 = row.try_get::<i64>("", "amount_minor_units").ok()?;
            let timestamp: DateTime<Utc> = row.try_get::<DateTime<Utc>>("", "created_at").ok()?;
            Some(RecentTransaction {
                transaction_id,
                amount_minor_units,
                payment_method_id: None,
                timestamp,
            })
        }).collect();

        Ok(txns)
    }

    async fn get_recent_by_method(&self, _payment_method_id: &str, _window_seconds: u32) -> Result<Vec<RecentTransaction>, ComplianceError> {
        // Stub — requires payment_method_token table linkage
        Ok(Vec::new())
    }

    async fn get_average_amount(&self, operator_id: Uuid, _min_sample_size: u32) -> Result<f64, ComplianceError> {
        // Query average transaction amount from payment_intents table
        let stmt = sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            r#"
            SELECT COALESCE(AVG(amount_minor_units::float8), 0.0) AS avg_amount
            FROM payment_intents
            WHERE operator_id = $1
            "#,
            vec![operator_id.into()],
        );

        let rows = self.db
            .query_all(stmt)
            .await
            .map_err(|e| ComplianceError::InvalidRequest(e.to_string()))?;

        let avg: f64 = rows.first()
            .and_then(|row| row.try_get::<f64>("", "avg_amount").ok())
            .unwrap_or(0.0);
        Ok(avg)
    }
}

// ─── Domain ↔ Model conversion: KybCase ──────────────────────────────────────

fn kyb_domain_to_model(k: &KybCase) -> Result<KybCaseModel, ComplianceError> {
    let document_ids = serde_json::to_value(&k.document_ids)
        .map_err(|e| ComplianceError::InvalidRequest(format!("Serialize document_ids: {}", e)))?;

    Ok(KybCaseModel {
        kyb_case_id: k.kyb_case_id,
        operator_id: k.operator_id,
        status: k.status.as_str().to_string(),
        submitted_by: k.submitted_by,
        document_ids,
        ocr_extracted_fields: k.ocr_extracted_fields.clone(),
        partner_decision: k.partner_decision.clone(),
        rejection_reason: k.rejection_reason.clone(),
        submitted_at: k.submitted_at,
        resolved_at: k.resolved_at,
        updated_at: k.updated_at,
    })
}

fn kyb_model_to_domain(m: KybCaseModel) -> Result<KybCase, ComplianceError> {
    let status = KybStatus::parse_str(&m.status)
        .ok_or_else(|| ComplianceError::InvalidRequest(format!("Unknown status: {}", m.status)))?;

    let document_ids: Vec<Uuid> = serde_json::from_value(m.document_ids)
        .map_err(|e| ComplianceError::InvalidRequest(format!("Deserialize document_ids: {}", e)))?;

    Ok(KybCase {
        kyb_case_id: m.kyb_case_id,
        operator_id: m.operator_id,
        status,
        submitted_by: m.submitted_by,
        document_ids,
        ocr_extracted_fields: m.ocr_extracted_fields,
        partner_decision: m.partner_decision,
        rejection_reason: m.rejection_reason,
        submitted_at: m.submitted_at,
        resolved_at: m.resolved_at,
        updated_at: m.updated_at,
    })
}

// ─── Domain ↔ Model conversion: AmlAlert ─────────────────────────────────────

fn aml_domain_to_model(a: &AmlAlert) -> Result<AmlAlertModel, ComplianceError> {
    Ok(AmlAlertModel {
        alert_id: a.alert_id,
        operator_id: a.operator_id,
        transaction_id: a.transaction_id,
        alert_type: a.alert_type.as_str().to_string(),
        severity: a.severity.as_str().to_string(),
        rule_id: a.rule_id.clone(),
        details: a.details.clone(),
        status: a.status.as_str().to_string(),
        reviewed_by: a.reviewed_by,
        reviewed_at: a.reviewed_at,
        created_at: a.created_at,
    })
}

fn aml_model_to_domain(m: AmlAlertModel) -> Result<AmlAlert, ComplianceError> {
    let alert_type = AmlAlertType::parse_str(&m.alert_type)
        .ok_or_else(|| ComplianceError::InvalidRequest(format!("Unknown alert_type: {}", m.alert_type)))?;

    let severity = AlertSeverity::parse_str(&m.severity)
        .ok_or_else(|| ComplianceError::InvalidRequest(format!("Unknown severity: {}", m.severity)))?;

    let status = AlertStatus::parse_str(&m.status)
        .ok_or_else(|| ComplianceError::InvalidRequest(format!("Unknown status: {}", m.status)))?;

    Ok(AmlAlert {
        alert_id: m.alert_id,
        operator_id: m.operator_id,
        transaction_id: m.transaction_id,
        alert_type,
        severity,
        rule_id: m.rule_id,
        details: m.details,
        status,
        reviewed_by: m.reviewed_by,
        reviewed_at: m.reviewed_at,
        created_at: m.created_at,
    })
}
