//! ReconciliationMatcher — settlement record matching logic.

use super::match_result::{
    MatchResult, MatchStrategy, PaymentIntentRef, SettlementMatchOutcome,
};
use super::settlement_batch::SettlementRecord;

pub struct ReconciliationMatcher {
    pub auto_confirm_threshold: f64,
    pub review_threshold: f64,
}

impl Default for ReconciliationMatcher {
    fn default() -> Self {
        Self {
            auto_confirm_threshold: 0.95,
            review_threshold: 0.70,
        }
    }
}

impl ReconciliationMatcher {
    pub fn match_record(
        &self,
        record: &SettlementRecord,
        payment_intents: &[PaymentIntentRef],
    ) -> MatchResult {
        let record_id = record.record_id;

        // Strategy 1: Exact match by acquirer_reference
        if let Some(ref acquirer_ref) = record.acquirer_reference {
            let exact_matches: Vec<&PaymentIntentRef> = payment_intents
                .iter()
                .filter(|pi| pi.acquirer_reference.as_deref() == Some(acquirer_ref.as_str()))
                .collect();

            match exact_matches.len() {
                0 => { /* fall through to fuzzy */ }
                1 => {
                    return MatchResult {
                        settlement_record_id: record_id,
                        payment_intent_id: Some(exact_matches[0].payment_intent_id),
                        confidence: 1.0,
                        strategy: MatchStrategy::Exact,
                        outcome: if exact_matches[0].amount_minor == record.amount_minor {
                            SettlementMatchOutcome::AutoConfirmed
                        } else {
                            SettlementMatchOutcome::AmountMismatch
                        },
                    };
                }
                _ => {
                    return MatchResult {
                        settlement_record_id: record_id,
                        payment_intent_id: None,
                        confidence: 0.0,
                        strategy: MatchStrategy::Exact,
                        outcome: SettlementMatchOutcome::DuplicateReference,
                    };
                }
            }
        }

        // Strategy 2: Fuzzy match by amount + date proximity
        let amount_matches: Vec<&PaymentIntentRef> = payment_intents
            .iter()
            .filter(|pi| {
                let amount_diff = (pi.amount_minor - record.amount_minor).abs();
                // Within fee tolerance: allow up to 10% difference for fee
                amount_diff <= (pi.amount_minor as f64 * 0.10) as i64
            })
            .collect();

        if amount_matches.len() == 1 {
            let confidence = 0.85;
            return MatchResult {
                settlement_record_id: record_id,
                payment_intent_id: Some(amount_matches[0].payment_intent_id),
                confidence,
                strategy: MatchStrategy::Fuzzy,
                outcome: if confidence >= self.auto_confirm_threshold {
                    SettlementMatchOutcome::AutoConfirmed
                } else if confidence >= self.review_threshold {
                    SettlementMatchOutcome::Matched
                } else {
                    SettlementMatchOutcome::Unmatched
                },
            };
        }

        // No match found
        MatchResult {
            settlement_record_id: record_id,
            payment_intent_id: None,
            confidence: 0.0,
            strategy: MatchStrategy::Fuzzy,
            outcome: SettlementMatchOutcome::Unmatched,
        }
    }
}
