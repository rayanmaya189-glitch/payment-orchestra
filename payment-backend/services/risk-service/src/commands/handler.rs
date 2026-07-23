//! Fraud & Risk Scoring command handlers — BC-11

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::*;
use crate::repository::*;
use crate::commands::types::*;

// ---------------------------------------------------------------------------
// Command handler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn assess_risk(&self, cmd: AssessRiskCommand) -> Result<RiskAssessment, RiskError>;
    async fn update_risk_rule(&self, cmd: UpdateRiskRuleCommand) -> Result<RiskRule, RiskError>;
    async fn get_default_rules(&self) -> Vec<RiskRule>;
}

// ---------------------------------------------------------------------------
// Handler implementation
// ---------------------------------------------------------------------------

pub struct RiskCommandHandler<R: RiskRepository> {
    repo: R,
    engine: RiskEngine,
}

impl<R: RiskRepository> RiskCommandHandler<R> {
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            engine: RiskEngine::new(),
        }
    }

    pub fn with_engine(repo: R, engine: RiskEngine) -> Self {
        Self { repo, engine }
    }
}

#[async_trait]
impl<R: RiskRepository + Send + Sync> CommandHandler for RiskCommandHandler<R> {
    async fn assess_risk(&self, cmd: AssessRiskCommand) -> Result<RiskAssessment, RiskError> {
        // Validate card BIN
        if cmd.card_bin.len() != 6 || !cmd.card_bin.chars().all(|c| c.is_ascii_digit()) {
            return Err(RiskError::InvalidCardBin(cmd.card_bin));
        }

        // Validate currency
        let supported = ["AED", "USD", "EUR", "GBP", "SAR"];
        if !supported.contains(&cmd.currency.as_str()) {
            return Err(RiskError::UnsupportedCurrency(cmd.currency));
        }

        let input = AssessRiskInput {
            payment_intent_id: cmd.payment_intent_id,
            amount_minor_units: cmd.amount_minor_units,
            currency: cmd.currency,
            card_bin: cmd.card_bin,
            billing_country: cmd.billing_country,
            shipping_country: cmd.shipping_country,
            is_new_payment_method: cmd.is_new_payment_method,
        };

        let assessment = self.engine.assess(&input);
        self.repo.save(&assessment).await?;
        Ok(assessment)
    }

    async fn update_risk_rule(&self, cmd: UpdateRiskRuleCommand) -> Result<RiskRule, RiskError> {
        let rules = RiskEngine::default_rules();
        let mut rule = rules
            .into_iter()
            .find(|r| r.rule_id == cmd.rule_id)
            .ok_or_else(|| {
                RiskError::NotFound(Uuid::nil())
            })?;

        if let Some(inc) = cmd.score_increment {
            rule.score_increment = inc;
        }
        if let Some(active) = cmd.is_active {
            rule.is_active = active;
        }
        if let Some(desc) = cmd.description {
            rule.description = desc;
        }

        Ok(rule)
    }

    async fn get_default_rules(&self) -> Vec<RiskRule> {
        RiskEngine::default_rules()
    }
}
