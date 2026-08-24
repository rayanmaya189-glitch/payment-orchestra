//! Read-model queries for compliance-service.

use uuid::Uuid;

use crate::domain::{KybCase, AmlAlert, ComplianceError};
use crate::repository::ComplianceRepository;

// Blanket impl: Box<dyn QueryHandler> implements QueryHandler
#[async_trait::async_trait]
impl QueryHandler for Box<dyn QueryHandler> {
    async fn get_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        self.as_ref().get_kyb_case(id).await
    }
    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError> {
        self.as_ref().list_pending_kyb_cases().await
    }
    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError> {
        self.as_ref().list_aml_alerts(operator_id, status_filter).await
    }
}

#[async_trait::async_trait]
pub trait QueryHandler: Send + Sync {
    async fn get_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError>;
    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError>;
    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError>;
}

pub struct ComplianceQueries<R: ComplianceRepository> {
    repository: R,
}

impl<R: ComplianceRepository> ComplianceQueries<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl<R: ComplianceRepository + Send + Sync> QueryHandler for ComplianceQueries<R> {
    async fn get_kyb_case(&self, id: Uuid) -> Result<Option<KybCase>, ComplianceError> {
        self.repository.load_kyb_case(id).await
    }

    async fn list_pending_kyb_cases(&self) -> Result<Vec<KybCase>, ComplianceError> {
        self.repository.list_pending_kyb_cases().await
    }

    async fn list_aml_alerts(&self, operator_id: Uuid, status_filter: Option<&str>) -> Result<Vec<AmlAlert>, ComplianceError> {
        self.repository.list_aml_alerts(operator_id, status_filter).await
    }
}
