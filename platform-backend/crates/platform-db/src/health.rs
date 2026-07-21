//! Health check utilities for all services.
//!
//! Provides /healthz, /readyz, and /startupz endpoints per SRS Part 4 §6.3.
//! - /healthz: Liveness check — is the process alive?
//! - /readyz: Readiness check — are dependencies (DB, Redis, NATS) reachable?
//! - /startupz: Startup check — has initialization completed?

use sea_orm::{DatabaseConnection, ConnectionTrait, Statement};
use redis::aio::ConnectionManager;

/// Health check result for a single dependency.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyHealth {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Overall health check response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub checks: Vec<DependencyHealth>,
    pub uptime_secs: u64,
}

/// Check PostgreSQL connectivity.
pub async fn check_postgres(db: &DatabaseConnection) -> DependencyHealth {
    let start = std::time::Instant::now();
    match db.execute(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT 1".to_string(),
    )).await {
        Ok(_) => DependencyHealth {
            name: "postgres".to_string(),
            status: "ok".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => DependencyHealth {
            name: "postgres".to_string(),
            status: "error".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    }
}

/// Check Redis connectivity.
pub async fn check_redis(redis: &ConnectionManager) -> DependencyHealth {
    let start = std::time::Instant::now();
    let mut conn = redis.clone();
    match redis::cmd("PING").query_async::<String>(&mut conn).await {
        Ok(_) => DependencyHealth {
            name: "redis".to_string(),
            status: "ok".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => DependencyHealth {
            name: "redis".to_string(),
            status: "error".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    }
}

/// Build a readiness response as serde_json::Value.
pub async fn readiness_json(
    service_name: &str,
    db: &DatabaseConnection,
    redis: &ConnectionManager,
    started_at: std::time::Instant,
) -> serde_json::Value {
    let mut checks = Vec::new();

    checks.push(check_postgres(db).await);
    checks.push(check_redis(redis).await);

    let all_healthy = checks.iter().all(|c| c.status == "ok");
    let status = if all_healthy { "ok" } else { "degraded" };

    serde_json::json!({
        "status": status,
        "service": service_name,
        "checks": checks,
        "uptime_secs": started_at.elapsed().as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_health_serialization() {
        let health = DependencyHealth {
            name: "postgres".to_string(),
            status: "ok".to_string(),
            latency_ms: Some(5),
            error: None,
        };
        let json = serde_json::to_value(&health).unwrap();
        assert_eq!(json["name"], "postgres");
        assert_eq!(json["status"], "ok");
        assert_eq!(json["latency_ms"], 5);
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            service: "test-service".to_string(),
            checks: vec![],
            uptime_secs: 100,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["uptime_secs"], 100);
    }
}
