use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresReconciliationRepository;
use crate::repository::FeeVarianceRepository;
use crate::domain::*;
use crate::entities::fee_variance::{
    Entity as FeeVarianceEntity,
    ActiveModel as FeeVarianceActiveModel,
    Model as FeeVarianceModel,
    Column as FeeVarianceColumn,
};

#[async_trait]
impl FeeVarianceRepository for PostgresReconciliationRepository {
    async fn save_fee_variance(
        &self,
        variance: &FeeVariance,
    ) -> Result<(), ReconciliationError> {
        let model = fee_variance_domain_to_model(variance)?;
        let exists = FeeVarianceEntity::find_by_id(variance.variance_id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            FeeVarianceEntity::update(FeeVarianceActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        } else {
            FeeVarianceEntity::insert(FeeVarianceActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn load_fee_variance(
        &self,
        id: Uuid,
    ) -> Result<Option<FeeVariance>, ReconciliationError> {
        let result = FeeVarianceEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(fee_variance_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_fee_variances_for_payment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Vec<FeeVariance>, ReconciliationError> {
        let models = FeeVarianceEntity::find()
            .filter(FeeVarianceColumn::PaymentIntentId.eq(payment_intent_id))
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        models.into_iter().map(fee_variance_model_to_domain).collect()
    }
}

fn fee_variance_domain_to_model(v: &FeeVariance) -> Result<FeeVarianceModel, ReconciliationError> {
    Ok(FeeVarianceModel {
        variance_id: v.variance_id,
        payment_intent_id: v.payment_intent_id,
        expected_fee_minor: v.estimated_fee_minor,
        actual_fee_minor: v.actual_fee_minor,
        variance_amount_minor: v.variance_minor,
        currency: "AED".to_string(),
        reason: v.resolution_note.clone(),
        created_at: v.detected_at,
    })
}

fn fee_variance_model_to_domain(m: FeeVarianceModel) -> Result<FeeVariance, ReconciliationError> {
    let variance_percent = if m.expected_fee_minor != 0 {
        (m.variance_amount_minor as f64 / m.expected_fee_minor as f64) * 100.0
    } else {
        0.0
    };

    Ok(FeeVariance {
        variance_id: m.variance_id,
        payment_intent_id: m.payment_intent_id,
        acquirer_link_id: Uuid::default(),
        estimated_fee_minor: m.expected_fee_minor,
        actual_fee_minor: m.actual_fee_minor,
        variance_minor: m.variance_amount_minor,
        variance_percent,
        is_within_tolerance: variance_percent.abs() <= 5.0,
        tolerance_threshold_percent: 5.0,
        status: FeeVarianceStatus::VarianceDetected,
        detected_at: m.created_at,
        resolved_at: None,
        resolution_note: m.reason,
    })
}
