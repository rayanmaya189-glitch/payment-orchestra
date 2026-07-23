//! Dispute Management API surface — BC-10

use crate::commands::*;
use crate::domain::{ChargebackCase, DisputeError};
use crate::queries::*;
use uuid::Uuid;

/// Public API facade for the dispute service.
pub mod grpc;

pub struct DisputeApi {
    command_handler: Box<dyn CommandHandler>,
    query_handler: Box<dyn QueryHandler>,
}

impl DisputeApi {
    pub fn new(
        command_handler: Box<dyn CommandHandler>,
        query_handler: Box<dyn QueryHandler>,
    ) -> Self {
        Self {
            command_handler,
            query_handler,
        }
    }

    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    pub async fn record_chargeback(
        &self,
        cmd: RecordChargebackCommand,
    ) -> Result<ChargebackCase, DisputeError> {
        self.command_handler.record_chargeback(cmd).await
    }

    pub async fn submit_representment(
        &self,
        cmd: SubmitRepresentmentCommand,
    ) -> Result<ChargebackCase, DisputeError> {
        self.command_handler.submit_representment(cmd).await
    }

    pub async fn resolve_chargeback(
        &self,
        cmd: ResolveChargebackCommand,
    ) -> Result<ChargebackCase, DisputeError> {
        self.command_handler.resolve_chargeback(cmd).await
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    pub async fn get_chargeback(&self, id: Uuid) -> Result<ChargebackCase, DisputeError> {
        self.query_handler.get_chargeback(id).await
    }

    pub async fn find_by_payment_intent(
        &self,
        payment_intent_id: Uuid,
    ) -> Result<Vec<ChargebackCase>, DisputeError> {
        self.query_handler.find_by_payment_intent(payment_intent_id).await
    }

    pub async fn find_open_cases(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<ChargebackCase>, DisputeError> {
        self.query_handler.find_open_cases(operator_id).await
    }

    pub async fn find_by_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<ChargebackCase>, DisputeError> {
        self.query_handler.find_by_operator(operator_id).await
    }
}
