//! AI Gateway commands

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn process_query(&self, cmd: ProcessAiQuery) -> Result<GuardrailResult, AiGatewayError>;
    async fn record_model_failure(&self) -> Result<(), AiGatewayError>;
    async fn record_model_success(&self) -> Result<(), AiGatewayError>;
    async fn reset_quota(&self, cmd: ResetQuota) -> Result<(), AiGatewayError>;
}

pub struct AiGatewayCommandHandler<R: AiGatewayRepository> {
    repo: R,
}

impl<R: AiGatewayRepository> AiGatewayCommandHandler<R> {
    pub fn new(repo: R) -> Self { Self { repo } }
}

#[async_trait]
impl<R: AiGatewayRepository + Send + Sync> CommandHandler for AiGatewayCommandHandler<R> {
    async fn process_query(&self, cmd: ProcessAiQuery) -> Result<GuardrailResult, AiGatewayError> {
        let query_id = Uuid::now_v7();

        let cb_state = self.repo.load_circuit_breaker().await?;
        if !cb_state.should_allow() {
            return Err(AiGatewayError::CircuitOpen);
        }

        let mut quota = self.repo.load_quota(cmd.operator_id).await?
            .unwrap_or_else(|| UsageQuota::new(cmd.operator_id, 100));
        if !quota.has_available() {
            return Err(AiGatewayError::QuotaExceeded);
        }

        let (blocked, block_reason, screening_score) = screen_prompt(&cmd.question);

        let model_route = if cmd.has_attachment {
            ModelRoute::VisionExtraction
        } else {
            ModelRoute::TextOnly
        };

        quota.record_query();
        self.repo.save_quota(&quota).await?;

        let audit_entry = GuardrailAuditEntry {
            audit_id: Uuid::now_v7(),
            query_id,
            operator_id: cmd.operator_id,
            blocked,
            block_reason: block_reason.clone(),
            screening_score,
            citations_valid: true,
            circuit_open: cb_state.is_open,
            quota_remaining: quota.remaining(),
            processed_at: Utc::now(),
        };
        self.repo.save_audit_entry(&audit_entry).await?;

        if blocked {
            return Err(AiGatewayError::QueryBlocked(block_reason.unwrap_or_default()));
        }

        Ok(GuardrailResult {
            query_id,
            blocked: false,
            block_reason: None,
            screening_score,
            model_route,
            citations_valid: true,
            quota_available: true,
            quota_remaining: quota.remaining(),
            circuit_open: cb_state.is_open,
        })
    }

    async fn record_model_failure(&self) -> Result<(), AiGatewayError> {
        let mut cb = self.repo.load_circuit_breaker().await?;
        cb.record_failure();
        self.repo.save_circuit_breaker(&cb).await?;
        Ok(())
    }

    async fn record_model_success(&self) -> Result<(), AiGatewayError> {
        let mut cb = self.repo.load_circuit_breaker().await?;
        cb.record_success();
        self.repo.save_circuit_breaker(&cb).await?;
        Ok(())
    }

    async fn reset_quota(&self, cmd: ResetQuota) -> Result<(), AiGatewayError> {
        let quota = UsageQuota::new(cmd.operator_id, 100);
        self.repo.save_quota(&quota).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Prompt injection screening
// ---------------------------------------------------------------------------

fn screen_prompt(question: &str) -> (bool, Option<String>, f64) {
    let lower = question.to_lowercase();
    let mut score = 0.0_f64;

    for pattern in INJECTION_PATTERNS {
        if lower.contains(pattern) {
            score += 0.3;
        }
    }

    if lower.matches('\n').count() > 10 {
        score += 0.2;
    }
    if lower.matches("{{").count() > 0 || lower.matches("{%").count() > 0 {
        score += 0.3;
    }
    if question.len() > 2000 {
        score += 0.1;
    }

    score = score.min(1.0);

    if score >= 0.5 {
        let reason = format!("Prompt injection score {:.2} exceeds threshold 0.5", score);
        (true, Some(reason), score)
    } else {
        (false, None, score)
    }
}
