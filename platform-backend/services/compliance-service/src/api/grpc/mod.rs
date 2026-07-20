//! gRPC server implementation for Compliance Service.
//!
//! Provides KYB and compliance operations via gRPC.

use uuid::Uuid;
use std::sync::Arc;

use crate::application::services::ComplianceService;
use platform_error::PlatformError;

/// gRPC server implementation wrapping the HTTP service.
pub struct GrpcComplianceService {
    service: Arc<dyn ComplianceService>,
}

impl GrpcComplianceService {
    pub fn new(service: Arc<dyn ComplianceService>) -> Self {
        Self { service }
    }

    /// Submit KYB evidence for an operator.
    pub async fn submit_kyb_evidence(
        &self,
        operator_id: Uuid,
        _document_ids: Vec<String>,
    ) -> Result<(Uuid, String), PlatformError> {
        let cmd = crate::application::commands::CreateKybCaseCommand {
            operator_id,
        };

        let result = self.service.create_kyb_case(cmd).await?;
        Ok((Uuid::nil(), result.status))
    }

    /// Review a KYB case.
    pub async fn review_kyb_case(
        &self,
        kyb_case_id: Uuid,
        decision: String,
        reason: String,
        decided_by: Uuid,
    ) -> Result<(), PlatformError> {
        let cmd = crate::application::commands::DecideKybCaseCommand {
            kyb_case_id,
            decision,
            reason,
            decided_by,
        };

        self.service.decide_case(cmd).await
    }

    /// Get a KYB case by ID.
    pub async fn get_kyb_case(
        &self,
        kyb_case_id: Uuid,
    ) -> Result<crate::api::dto::KybCaseResponse, PlatformError> {
        let query = crate::application::queries::GetKybCaseQuery { kyb_case_id };
        self.service.get_case(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_compliance_service_creation() {
        // Verify module compiles
    }
}
