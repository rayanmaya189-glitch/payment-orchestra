use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::api::dto::KybCaseResponse;
use crate::domain::aggregates::{KybCase, KybDocument};
use crate::domain::value_objects::{KybCaseStatus, KybDocumentType};
use crate::infrastructure::repository::KybCaseRepository;
use platform_error::PlatformError;

#[async_trait]
pub trait ComplianceService: Send + Sync {
    async fn create_kyb_case(&self, cmd: CreateKybCaseCommand) -> Result<KybCaseResponse, PlatformError>;
    async fn upload_document(&self, cmd: UploadDocumentCommand) -> Result<(), PlatformError>;
    async fn verify_document(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError>;
    async fn assign_officer(&self, cmd: AssignOfficerCommand) -> Result<(), PlatformError>;
    async fn decide_case(&self, cmd: DecideKybCaseCommand) -> Result<(), PlatformError>;
    async fn request_documents(&self, cmd: RequestDocumentsCommand) -> Result<(), PlatformError>;
    async fn get_case(&self, query: GetKybCaseQuery) -> Result<KybCaseResponse, PlatformError>;
    async fn list_cases(&self, query: ListKybCasesQuery) -> Result<Vec<KybCaseResponse>, PlatformError>;
}

pub struct ComplianceServiceImpl {
    repo: Box<dyn KybCaseRepository>,
    db: sea_orm::DatabaseConnection,
}

impl ComplianceServiceImpl {
    pub fn new(
        repo: Box<dyn KybCaseRepository>,
        db: sea_orm::DatabaseConnection,
    ) -> Self {
        Self { repo, db }
    }
}

#[async_trait]
impl ComplianceService for ComplianceServiceImpl {
    async fn create_kyb_case(&self, cmd: CreateKybCaseCommand) -> Result<KybCaseResponse, PlatformError> {
        // Check if operator already has an active KYB case
        if let Some(existing) = self.repo.find_by_operator(cmd.operator_id).await? {
            if matches!(existing.status, KybCaseStatus::Submitted | KybCaseStatus::UnderReview | KybCaseStatus::DocumentsRequested) {
                return Err(platform_error::PlatformError::Conflict(
                    platform_error::ConflictError::DuplicateOrderInvoice
                ));
            }
        }

        let kyb_case = KybCase::new(cmd.operator_id);
        self.repo.save(&kyb_case).await?;

        Ok(case_to_response(&kyb_case))
    }

    async fn upload_document(&self, cmd: UploadDocumentCommand) -> Result<(), PlatformError> {
        let mut case = self.repo
            .load(cmd.kyb_case_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "KYB Case".into(),
                id: cmd.kyb_case_id,
            })?;

        // Validate case is in a state that accepts documents
        if !matches!(case.status, KybCaseStatus::Submitted | KybCaseStatus::DocumentsRequested | KybCaseStatus::UnderReview) {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: case.status.as_str().to_string(),
                    command: "UploadDocument".to_string(),
                }
            ));
        }

        let document_type = KybDocumentType::from_str(&cmd.document_type);

        let document = KybDocument {
            id: Uuid::now_v7(),
            kyb_case_id: cmd.kyb_case_id,
            document_type,
            file_key: cmd.file_key,
            file_hash: cmd.file_hash,
            uploaded_at: Utc::now(),
            verified: false,
            verified_at: None,
        };

        self.repo.save_document(&document).await?;

        // Update case status to UnderReview if it was Submitted
        if case.status == KybCaseStatus::Submitted {
            case.status = KybCaseStatus::UnderReview;
            self.repo.save(&case).await?;
        }

        Ok(())
    }

    async fn verify_document(&self, cmd: VerifyDocumentCommand) -> Result<(), PlatformError> {
        self.repo.update_document_verification(cmd.document_id, cmd.verified).await?;

        // Check if all documents for the case are now verified
        // This would require loading the case and checking all documents
        // For now, just update the document

        Ok(())
    }

    async fn assign_officer(&self, cmd: AssignOfficerCommand) -> Result<(), PlatformError> {
        let mut case = self.repo
            .load(cmd.kyb_case_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "KYB Case".into(),
                id: cmd.kyb_case_id,
            })?;

        case.assigned_compliance_officer = Some(cmd.officer_id);
        case.status = KybCaseStatus::UnderReview;
        self.repo.save(&case).await?;

        Ok(())
    }

    async fn decide_case(&self, cmd: DecideKybCaseCommand) -> Result<(), PlatformError> {
        let mut case = self.repo
            .load(cmd.kyb_case_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "KYB Case".into(),
                id: cmd.kyb_case_id,
            })?;

        // Validate case can be decided
        if !case.can_be_decided() {
            return Err(PlatformError::Validation(
                platform_error::ValidationError::InvalidStateTransition {
                    from: case.status.as_str().to_string(),
                    command: "Decide".to_string(),
                }
            ));
        }

        let decision_status = KybCaseStatus::from_str(&cmd.decision);
        case.status = decision_status.clone();
        case.decision = Some(crate::domain::aggregates::KybDecision {
            decision: decision_status,
            reason: cmd.reason.clone(),
            decided_by: cmd.decided_by,
            decided_at: Utc::now(),
        });
        case.decided_at = Some(Utc::now());

        self.repo.save(&case).await?;

        Ok(())
    }

    async fn request_documents(&self, cmd: RequestDocumentsCommand) -> Result<(), PlatformError> {
        let mut case = self.repo
            .load(cmd.kyb_case_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "KYB Case".into(),
                id: cmd.kyb_case_id,
            })?;

        case.status = KybCaseStatus::DocumentsRequested;
        case.notes = Some(cmd.reason);
        self.repo.save(&case).await?;

        Ok(())
    }

    async fn get_case(&self, query: GetKybCaseQuery) -> Result<KybCaseResponse, PlatformError> {
        let case = self.repo
            .load(query.kyb_case_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "KYB Case".into(),
                id: query.kyb_case_id,
            })?;

        Ok(case_to_response(&case))
    }

    async fn list_cases(&self, query: ListKybCasesQuery) -> Result<Vec<KybCaseResponse>, PlatformError> {
        let limit = query.limit.unwrap_or(20).min(100) as u64;
        let cases = self.repo
            .list(query.status.as_deref(), limit, 0)
            .await?;

        Ok(cases.iter().map(case_to_response).collect())
    }
}

fn case_to_response(case: &KybCase) -> KybCaseResponse {
    KybCaseResponse {
        id: case.id,
        operator_id: case.operator_id,
        status: case.status.as_str().to_string(),
        assigned_officer: case.assigned_compliance_officer,
        document_count: case.documents.len(),
        verified_count: case.documents.iter().filter(|d| d.verified).count(),
        risk_score: case.risk_score,
        submitted_at: case.submitted_at.to_rfc3339(),
        decided_at: case.decided_at.map(|dt| dt.to_rfc3339()),
    }
}
