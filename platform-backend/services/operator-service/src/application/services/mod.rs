use async_trait::async_trait;
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::api::dto::OperatorResponse;
use crate::domain::aggregates::Operator;
use crate::domain::value_objects::TradeLicenseNo;
use crate::infrastructure::messaging::EventPublisher;
use crate::infrastructure::repository::OperatorRepository;
use platform_error::{PlatformError, ConflictError};
use platform_middleware::{evaluate_policy, AbacContext};
use shared_types::events::EventEnvelope;
use shared_types::ActorType;

#[async_trait]
pub trait OperatorService: Send + Sync {
    async fn register_operator(&self, cmd: RegisterOperatorCommand) -> Result<OperatorResponse, PlatformError>;
    async fn verify_email(&self, cmd: VerifyEmailCommand) -> Result<(), PlatformError>;
    async fn update_status(&self, cmd: UpdateOperatorStatusCommand) -> Result<(), PlatformError>;
    async fn get_operator(&self, query: GetOperatorQuery) -> Result<OperatorResponse, PlatformError>;
    async fn list_operators(&self, query: ListOperatorsQuery) -> Result<Vec<OperatorResponse>, PlatformError>;
}

pub struct OperatorServiceImpl {
    repo: Box<dyn OperatorRepository>,
    event_publisher: EventPublisher,
    db: sea_orm::DatabaseConnection,
}

impl OperatorServiceImpl {
    pub fn new(
        repo: Box<dyn OperatorRepository>,
        event_publisher: EventPublisher,
        db: sea_orm::DatabaseConnection,
    ) -> Self {
        Self { repo, event_publisher, db }
    }

    /// Check ABAC permission (SRS ABAC-001 through ABAC-008).
    fn check_abac(
        &self,
        principal_id: Uuid,
        role: &str,
        action: &str,
        resource: &str,
    ) -> Result<(), PlatformError> {
        let ctx = AbacContext {
            principal_id,
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        evaluate_policy(&ctx)
    }
}

#[async_trait]
impl OperatorService for OperatorServiceImpl {
    async fn register_operator(&self, cmd: RegisterOperatorCommand) -> Result<OperatorResponse, PlatformError> {
        // ABAC: only platform_admin can register operators
        self.check_abac(cmd.principal_id, &cmd.role, "create", "operator")?;

        // Validate trade license format
        let trade_license = TradeLicenseNo::new(&cmd.trade_license_no)?;

        // Check duplicate trade license
        if self.repo.find_by_trade_license(&cmd.trade_license_no).await?.is_some() {
            return Err(PlatformError::Conflict(ConflictError::IdempotencyKeyConflict));
        }

        // Check duplicate email
        if self.repo.find_by_email(&cmd.email).await?.is_some() {
            return Err(PlatformError::Conflict(ConflictError::IdempotencyKeyConflict));
        }

        // Generate subdomain from legal name
        let subdomain = generate_subdomain(&cmd.legal_name);

        // Check subdomain uniqueness
        if self.repo.find_by_subdomain(&subdomain).await?.is_some() {
            return Err(PlatformError::Conflict(ConflictError::IdempotencyKeyConflict));
        }

        // Create operator aggregate
        let operator = Operator::new(
            cmd.legal_name,
            trade_license,
            cmd.country,
            cmd.email,
            subdomain,
        );

        // Persist
        self.repo.save(&operator).await?;

        // Publish domain event
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "Operator",
            operator.id,
            "OperatorRegistered",
            ActorType::System.as_str(),
            correlation_id,
            serde_json::json!({
                "operator_id": operator.id,
                "legal_name": operator.legal_name,
                "trade_license_no": operator.trade_license_no.to_string(),
                "country": operator.country,
                "email": operator.email,
                "subdomain": operator.subdomain,
            }),
        );

        self.event_publisher
            .publish_with_outbox(&self.db, &event, "Operator", operator.id)
            .await?;

        Ok(operator_to_response(&operator))
    }

    async fn verify_email(&self, cmd: VerifyEmailCommand) -> Result<(), PlatformError> {
        // ABAC: only platform_admin or compliance_officer can verify emails
        self.check_abac(cmd.principal_id, &cmd.role, "verify", "operator")?;

        let mut operator = self.repo
            .load(cmd.operator_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Operator".into(),
                id: cmd.operator_id,
            })?;

        // Domain rule: must be in Pending status
        operator.verify_email()?;

        // Persist
        self.repo.save(&operator).await?;

        // Publish event
        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "Operator",
            operator.id,
            "OperatorEmailVerified",
            ActorType::System.as_str(),
            correlation_id,
            serde_json::json!({
                "operator_id": operator.id,
                "new_status": operator.status.as_str(),
            }),
        );

        self.event_publisher
            .publish_with_outbox(&self.db, &event, "Operator", operator.id)
            .await?;

        Ok(())
    }

    async fn update_status(&self, cmd: UpdateOperatorStatusCommand) -> Result<(), PlatformError> {
        // ABAC: only platform_admin can change operator status
        self.check_abac(cmd.principal_id, &cmd.role, "update_status", "operator")?;

        let mut operator = self.repo
            .load(cmd.operator_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Operator".into(),
                id: cmd.operator_id,
            })?;

        let new_status = crate::domain::value_objects::OperatorStatus::from_str(&cmd.new_status)
            .ok_or_else(|| PlatformError::Validation(
                platform_error::ValidationError::MissingField(format!(
                    "Invalid status: {}", cmd.new_status
                ))
            ))?;

        operator.status = new_status.clone();
        self.repo.save(&operator).await?;

        let event_type = match new_status {
            crate::domain::value_objects::OperatorStatus::ActiveVerified => "OperatorVerified",
            crate::domain::value_objects::OperatorStatus::Suspended => "OperatorSuspended",
            _ => "OperatorStatusChanged",
        };

        let correlation_id = Uuid::now_v7();
        let event = EventEnvelope::new(
            "Operator",
            operator.id,
            event_type,
            ActorType::System.as_str(),
            correlation_id,
            serde_json::json!({
                "operator_id": operator.id,
                "old_status": "unknown",
                "new_status": new_status.as_str(),
                "reason": cmd.reason,
            }),
        );

        self.event_publisher
            .publish_with_outbox(&self.db, &event, "Operator", operator.id)
            .await?;

        Ok(())
    }

    async fn get_operator(&self, query: GetOperatorQuery) -> Result<OperatorResponse, PlatformError> {
        let operator = self.repo
            .load(query.operator_id)
            .await?
            .ok_or_else(|| PlatformError::NotFound {
                resource: "Operator".into(),
                id: query.operator_id,
            })?;

        Ok(operator_to_response(&operator))
    }

    async fn list_operators(&self, query: ListOperatorsQuery) -> Result<Vec<OperatorResponse>, PlatformError> {
        let limit = query.limit.unwrap_or(20).min(100) as u64;
        let operators = self.repo
            .list(query.status.as_deref(), limit, 0)
            .await?;

        Ok(operators.iter().map(operator_to_response).collect())
    }
}

fn operator_to_response(op: &Operator) -> OperatorResponse {
    OperatorResponse {
        id: op.id,
        legal_name: op.legal_name.clone(),
        trade_license_no: op.trade_license_no.to_string(),
        country: op.country.clone(),
        status: op.status.as_str().to_string(),
        subdomain: op.subdomain.clone(),
        email: op.email.clone(),
        provisioned_at: op.provisioned_at.map(|dt| dt.to_rfc3339()),
        created_at: op.created_at.to_rfc3339(),
    }
}

fn generate_subdomain(legal_name: &str) -> String {
    let subdomain = legal_name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Truncate to 63 chars (DNS limit per RFC 1035)
    let subdomain = if subdomain.len() > 63 {
        subdomain[..63].trim_end_matches('-').to_string()
    } else {
        subdomain
    };

    // DNS validation: must not start/end with hyphen, must be 1-63 chars
    // Only allows [a-z0-9-] (already filtered above)
    validate_subdomain(&subdomain).then_some(subdomain).unwrap_or_default()
}

/// Validate a subdomain string against DNS naming rules (RFC 1035).
/// - 1-63 characters
/// - Only [a-z0-9-]
/// - Must not start or end with hyphen
/// - Must not contain consecutive hyphens
fn validate_subdomain(subdomain: &str) -> bool {
    if subdomain.is_empty() || subdomain.len() > 63 {
        return false;
    }
    if subdomain.starts_with('-') || subdomain.ends_with('-') {
        return false;
    }
    if subdomain.contains("--") {
        return false;
    }
    subdomain.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_subdomain() {
        assert_eq!(generate_subdomain("Acme Corp"), "acme-corp");
        assert_eq!(generate_subdomain("Al Futtaim Group"), "al-futtaim-group");
        assert_eq!(generate_subdomain("  Spaces  "), "spaces");
    }

    #[test]
    fn test_validate_subdomain() {
        assert!(validate_subdomain("acme-corp"));
        assert!(validate_subdomain("a"));
        assert!(validate_subdomain("test123"));
        assert!(!validate_subdomain("")); // empty
        assert!(!validate_subdomain("-start")); // starts with hyphen
        assert!(!validate_subdomain("end-")); // ends with hyphen
        assert!(!validate_subdomain("double--hyphen")); // consecutive hyphens
        assert!(!validate_subdomain(&"a".repeat(64))); // too long
    }
}
