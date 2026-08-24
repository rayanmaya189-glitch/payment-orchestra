use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::PostgresReconciliationRepository;
use crate::repository::SettlementExpectationRepository;
use crate::domain::*;
use crate::entities::settlement_expectation::{
    Entity as SettlementExpectationEntity,
    ActiveModel as SettlementExpectationActiveModel,
    Model as SettlementExpectationModel,
    Column as SettlementExpectationColumn,
};

#[async_trait]
impl SettlementExpectationRepository for PostgresReconciliationRepository {
    async fn save_settlement_expectation(
        &self,
        expectation: &SettlementExpectation,
    ) -> Result<(), ReconciliationError> {
        let model = settlement_expectation_domain_to_model(expectation);
        let exists = SettlementExpectationEntity::find_by_id(expectation.expectation_id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?
            .is_some();

        if exists {
            SettlementExpectationEntity::update(SettlementExpectationActiveModel::from(model.clone()))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        } else {
            SettlementExpectationEntity::insert(SettlementExpectationActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn load_settlement_expectation(
        &self,
        id: Uuid,
    ) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let result = SettlementExpectationEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_expectation_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_expectation_by_payment(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Option<SettlementExpectation>, ReconciliationError> {
        let result = SettlementExpectationEntity::find()
            .filter(SettlementExpectationColumn::PaymentIntentId.eq(payment_intent_id))
            .one(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        match result {
            Some(m) => Ok(Some(settlement_expectation_model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn find_overdue_expectations(
        &self,
    ) -> Result<Vec<SettlementExpectation>, ReconciliationError> {
        let now = Utc::now();
        let models = SettlementExpectationEntity::find()
            .filter(SettlementExpectationColumn::Status.eq("pending"))
            .filter(SettlementExpectationColumn::ExpectedSettlementDate.lt(now))
            .all(&self.db)
            .await
            .map_err(|e| ReconciliationError::DatabaseError(e.to_string()))?;
        models.into_iter().map(settlement_expectation_model_to_domain).collect()
    }
}

fn settlement_expectation_domain_to_model(exp: &SettlementExpectation) -> SettlementExpectationModel {
    SettlementExpectationModel {
        expectation_id: exp.expectation_id,
        payment_intent_id: exp.payment_intent_id,
        expected_amount_minor: exp.settled_amount_minor.unwrap_or(0),
        currency: "AED".to_string(),
        expected_settlement_date: exp.expected_settlement_date,
        status: match exp.status {
            ExpectationStatus::Pending => "pending",
            ExpectationStatus::Settled => "settled",
            ExpectationStatus::Overdue => "overdue",
            ExpectationStatus::Adjusted => "adjusted",
        }.to_string(),
        created_at: exp.created_at,
    }
}

fn settlement_expectation_model_to_domain(m: SettlementExpectationModel) -> Result<SettlementExpectation, ReconciliationError> {
    let status = match m.status.as_str() {
        "pending" => ExpectationStatus::Pending,
        "settled" => ExpectationStatus::Settled,
        "overdue" => ExpectationStatus::Overdue,
        "adjusted" => ExpectationStatus::Adjusted,
        _ => ExpectationStatus::Pending,
    };

    Ok(SettlementExpectation {
        expectation_id: m.expectation_id,
        payment_intent_id: m.payment_intent_id,
        acquirer_link_id: Uuid::default(),
        expected_settlement_date: m.expected_settlement_date,
        settlement_cycle: "T+1".to_string(),
        status,
        settled_amount_minor: Some(m.expected_amount_minor),
        settled_at: None,
        created_at: m.created_at,
    })
}
