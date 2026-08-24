//! Fraud detection repository layer

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::{FraudCheckResult, TransactionContext};

/// Repository trait for fraud detection data
#[async_trait]
pub trait FraudRepository: Send + Sync {
    /// Store a fraud check result
    async fn store_fraud_check(&self, result: &FraudCheckResult) -> Result<(), RepositoryError>;

    /// Get recent transactions for velocity checks
    async fn get_recent_transactions(
        &self,
        customer_key: &str,
        window: Duration,
    ) -> Result<Vec<TransactionRecord>, RepositoryError>;

    /// Get users associated with a device
    async fn get_device_users(&self, device_id: &str) -> Result<Vec<String>, RepositoryError>;

    /// Get account age for email reputation
    async fn get_account_age(&self, email: &str) -> Result<Duration, RepositoryError>;

    /// Store transaction for future reference
    async fn store_transaction(&self, record: &TransactionRecord) -> Result<(), RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct TransactionRecord {
    pub transaction_id: Uuid,
    pub customer_key: String,
    pub amount_minor: i64,
    pub currency: String,
    pub device_id: Option<String>,
    pub created_at: Instant,
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

/// In-memory implementation for testing
pub struct InMemoryFraudRepository {
    fraud_checks: Arc<tokio::sync::RwLock<Vec<FraudCheckResult>>>,
    transactions: Arc<tokio::sync::RwLock<HashMap<String, Vec<TransactionRecord>>>>,
    device_users: Arc<tokio::sync::RwLock<HashMap<String, Vec<String>>>>,
    account_ages: Arc<tokio::sync::RwLock<HashMap<String, Instant>>>,
}

impl InMemoryFraudRepository {
    pub fn new() -> Self {
        Self {
            fraud_checks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            transactions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            device_users: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            account_ages: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryFraudRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FraudRepository for InMemoryFraudRepository {
    async fn store_fraud_check(&self, result: &FraudCheckResult) -> Result<(), RepositoryError> {
        let mut checks = self.fraud_checks.write().await;
        checks.push(result.clone());
        Ok(())
    }

    async fn get_recent_transactions(
        &self,
        customer_key: &str,
        window: Duration,
    ) -> Result<Vec<TransactionRecord>, RepositoryError> {
        let txns = self.transactions.read().await;
        let now = Instant::now();
        
        let recent = txns.get(customer_key)
            .map(|records| {
                records.iter()
                    .filter(|r| now.duration_since(r.created_at) <= window)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Ok(recent)
    }

    async fn get_device_users(&self, device_id: &str) -> Result<Vec<String>, RepositoryError> {
        let users = self.device_users.read().await;
        Ok(users.get(device_id).cloned().unwrap_or_default())
    }

    async fn get_account_age(&self, email: &str) -> Result<Duration, RepositoryError> {
        let ages = self.account_ages.read().await;
        Ok(ages.get(email)
            .map(|created| created.elapsed())
            .unwrap_or(Duration::from_secs(0)))
    }

    async fn store_transaction(&self, record: &TransactionRecord) -> Result<(), RepositoryError> {
        let mut txns = self.transactions.write().await;
        txns.entry(record.customer_key.clone())
            .or_insert_with(Vec::new)
            .push(record.clone());

        // Update device users
        if let Some(device_id) = &record.device_id {
            let mut devices = self.device_users.write().await;
            devices.entry(device_id.clone())
                .or_insert_with(Vec::new)
                .push(record.customer_key.clone());
        }

        Ok(())
    }
}

/// PostgreSQL implementation for production
pub struct PgFraudRepository {
    pool: sqlx::PgPool,
}

impl PgFraudRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FraudRepository for PgFraudRepository {
    async fn store_fraud_check(&self, result: &FraudCheckResult) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO fraud_checks (transaction_id, risk_score, decision, reasons, model_version, checked_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(result.transaction_id)
        .bind(result.risk_score)
        .bind(serde_json::to_string(&result.decision).unwrap())
        .bind(serde_json::to_string(&result.reasons).unwrap())
        .bind(&result.model_version)
        .bind(result.checked_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_recent_transactions(
        &self,
        customer_key: &str,
        window: Duration,
    ) -> Result<Vec<TransactionRecord>, RepositoryError> {
        let since = Utc::now() - chrono::Duration::from_std(window).unwrap();
        
        let rows = sqlx::query(
            r#"
            SELECT transaction_id, customer_key, amount_minor, currency, device_id, created_at
            FROM transactions
            WHERE customer_key = $1 AND created_at >= $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(customer_key)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        let records = rows.into_iter().map(|row| {
            TransactionRecord {
                transaction_id: row.get("transaction_id"),
                customer_key: row.get("customer_key"),
                amount_minor: row.get("amount_minor"),
                currency: row.get("currency"),
                device_id: row.get("device_id"),
                created_at: Instant::now(), // Simplified - in production use row.get("created_at")
            }
        }).collect();

        Ok(records)
    }

    async fn get_device_users(&self, device_id: &str) -> Result<Vec<String>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT customer_key
            FROM transactions
            WHERE device_id = $1
            ORDER BY customer_key
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        let users = rows.into_iter().map(|row| row.get("customer_key")).collect();
        Ok(users)
    }

    async fn get_account_age(&self, email: &str) -> Result<Duration, RepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT MIN(created_at) as first_seen
            FROM transactions
            WHERE customer_key = $1
            "#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        let first_seen: Option<chrono::DateTime<Utc>> = row.get("first_seen");
        match first_seen {
            Some(first_seen) => {
                let age = Utc::now() - first_seen;
                Ok(age.to_std().unwrap_or(Duration::ZERO))
            }
            None => Ok(Duration::ZERO),
        }
    }

    async fn store_transaction(&self, record: &TransactionRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO transactions (transaction_id, customer_key, amount_minor, currency, device_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (transaction_id) DO NOTHING
            "#,
        )
        .bind(record.transaction_id)
        .bind(&record.customer_key)
        .bind(record.amount_minor)
        .bind(&record.currency)
        .bind(&record.device_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        Ok(())
    }
}
