use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Status of an individual health check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Result of a single health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    #[serde(rename = "latencyMs")]
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Overall health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: HealthStatus,
    pub version: String,
    #[serde(rename = "uptimeSeconds")]
    pub uptime_seconds: u64,
    pub checks: Vec<CheckResult>,
}

impl HealthCheckResponse {
    pub fn new(version: &str, uptime_seconds: u64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            version: version.to_string(),
            uptime_seconds,
            checks: Vec::new(),
        }
    }

    pub fn add_check(&mut self, result: CheckResult) {
        if result.status != HealthStatus::Healthy {
            self.status = HealthStatus::Degraded;
        }
        if result.status == HealthStatus::Unhealthy {
            self.status = HealthStatus::Unhealthy;
        }
        self.checks.push(result);
    }
}

/// Check database connectivity and response time.
pub async fn check_database(db: &DatabaseConnection) -> CheckResult {
    let start = Instant::now();
    let result = db.execute(sea_orm::Statement::from_string(
        DbBackend::Postgres,
        String::from("SELECT 1"),
    )).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => CheckResult {
            name: "database".to_string(),
            status: if latency_ms < 100 {
                HealthStatus::Healthy
            } else if latency_ms < 500 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Unhealthy
            },
            message: format!("PostgreSQL responding ({}ms)", latency_ms),
            latency_ms,
            details: None,
        },
        Err(e) => CheckResult {
            name: "database".to_string(),
            status: HealthStatus::Unhealthy,
            message: format!("Database connection failed: {}", e),
            latency_ms,
            details: None,
        },
    }
}

/// Check Redis connectivity and response time.
pub async fn check_redis(client: &redis::Client) -> CheckResult {
    let start = Instant::now();
    let result = async {
        let mut conn = client.get_multiplexed_async_connection().await?;
        redis::cmd("PING").query_async::<String>(&mut conn).await
    }.await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => CheckResult {
            name: "redis".to_string(),
            status: if latency_ms < 10 {
                HealthStatus::Healthy
            } else if latency_ms < 50 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Unhealthy
            },
            message: format!("Redis responding ({}ms)", latency_ms),
            latency_ms,
            details: None,
        },
        Err(e) => CheckResult {
            name: "redis".to_string(),
            status: HealthStatus::Unhealthy,
            message: format!("Redis connection failed: {}", e),
            latency_ms,
            details: None,
        },
    }
}

/// Check NATS connectivity.
pub fn check_nats(client: &async_nats::Client) -> CheckResult {
    let start = Instant::now();
    // NATS server_info() is synchronous and always returns ServerInfo
    let info = client.server_info();
    let latency_ms = start.elapsed().as_millis() as u64;

    CheckResult {
        name: "nats".to_string(),
        status: HealthStatus::Healthy,
        message: format!("NATS responding ({}ms)", latency_ms),
        latency_ms,
        details: Some(serde_json::json!({
            "server_id": info.server_id,
            "server_name": info.server_name,
            "version": info.version,
            "host": info.host,
            "port": info.port,
        })),
    }
}
