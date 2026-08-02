//! Fraud detection gRPC service

use std::collections::HashMap;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::domain::{
    Address, DeviceFingerprint, FraudCheckResult, FraudDecision, FraudDetector,
    FraudReason, TransactionContext,
};

pub struct FraudGrpcService {
    detector: Arc<FraudDetector>,
}

impl FraudGrpcService {
    pub fn new(detector: FraudDetector) -> Self {
        Self {
            detector: Arc::new(detector),
        }
    }
}

// Proto-generated types would go here
// For now, using simplified request/response types

#[derive(Debug, Clone)]
pub struct CheckTransactionRequest {
    pub transaction_id: String,
    pub merchant_id: String,
    pub amount_minor: i64,
    pub currency: String,
    pub card_bin: Option<String>,
    pub card_last_four: Option<String>,
    pub card_country: Option<String>,
    pub customer_ip: Option<String>,
    pub customer_email: Option<String>,
    pub device_id: Option<String>,
    pub billing_country: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CheckTransactionResponse {
    pub transaction_id: String,
    pub risk_score: f64,
    pub decision: String,
    pub reasons: Vec<FraudReasonResponse>,
    pub model_version: String,
}

#[derive(Debug, Clone)]
pub struct FraudReasonResponse {
    pub rule: String,
    pub score_impact: f64,
    pub description: String,
}

impl FraudGrpcService {
    pub async fn check_transaction(
        &self,
        request: CheckTransactionRequest,
    ) -> Result<CheckTransactionResponse, Status> {
        let transaction_id = Uuid::parse_str(&request.transaction_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid transaction_id: {}", e)))?;

        let merchant_id = Uuid::parse_str(&request.merchant_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid merchant_id: {}", e)))?;

        let ctx = TransactionContext {
            transaction_id,
            merchant_id,
            amount_minor: request.amount_minor,
            currency: request.currency,
            card_bin: request.card_bin,
            card_last_four: request.card_last_four,
            card_country: request.card_country,
            customer_ip: request.customer_ip,
            customer_email: request.customer_email,
            device_fingerprint: request.device_id.map(|id| DeviceFingerprint {
                device_id: id,
                browser_type: None,
                os_type: None,
                screen_resolution: None,
                timezone: None,
                language: None,
                plugins: vec![],
            }),
            shipping_address: None,
            billing_address: request.billing_country.map(|country| Address {
                country,
                region: None,
                city: None,
                postal_code: None,
            }),
            metadata: HashMap::new(),
        };

        let result = self.detector
            .check_transaction(&ctx)
            .await
            .map_err(|e| Status::internal(format!("Fraud check failed: {}", e)))?;

        Ok(CheckTransactionResponse {
            transaction_id: result.transaction_id.to_string(),
            risk_score: result.risk_score,
            decision: format!("{:?}", result.decision),
            reasons: result.reasons.into_iter().map(|r| FraudReasonResponse {
                rule: r.rule,
                score_impact: r.score_impact,
                description: r.description,
            }).collect(),
            model_version: result.model_version,
        })
    }
}
