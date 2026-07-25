//! PostgreSQL repository for Scheduler — hybrid PG + in-memory.
//!
//! The `scheduled_jobs` entity stores a subset of the domain fields.
//! Fields missing from the entity (`job_key`, `service_name`, `description`,
//! `last_result`) are kept in a supplementary in-memory map.
//! `JobExecution` and `LeaderLease` have no corresponding entity tables,
//! so they use in-memory storage (same as the in-memory repo).

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::*;
use crate::entities::scheduled_job::{self, Entity as ScheduledJobEntity, Column as ScheduledJobColumn};
use crate::repository::SchedulerRepository;

#[derive(Clone)]
pub struct PostgresSchedulerRepository {
    db: sea_orm::DatabaseConnection,
    /// Supplementary fields not in the entity.
    extras: Arc<RwLock<HashMap<Uuid, JobExtras>>>,
    /// Executions — no entity table exists.
    executions: Arc<RwLock<Vec<JobExecution>>>,
    /// Leases — in-memory leader election.
    leases: Arc<RwLock<HashMap<String, LeaderLease>>>,
}

/// Fields that the entity lacks.
#[derive(Clone)]
struct JobExtras {
    job_key: String,
    service_name: String,
    description: String,
    schedule: JobSchedule,
    last_result: Option<JobRunResult>,
}

impl PostgresSchedulerRepository {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self {
            db,
            extras: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SchedulerRepository for PostgresSchedulerRepository {
    async fn load_job(&self, job_id: Uuid) -> Result<Option<ScheduledJob>, SchedulerError> {
        let result = ScheduledJobEntity::find_by_id(job_id)
            .one(&self.db)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let extra = self.extras.read().await.get(&job_id).cloned()
                    .unwrap_or_else(|| JobExtras {
                        job_key: String::new(),
                        service_name: String::new(),
                        description: String::new(),
                        schedule: JobSchedule::Every { interval_seconds: 3600 },
                        last_result: None,
                    });
                Ok(Some(model_to_domain(model, Some(extra))?))
            }
            None => Ok(None),
        }
    }

    async fn load_job_by_key(&self, job_key: &str) -> Result<Option<ScheduledJob>, SchedulerError> {
        let extras = self.extras.read().await;
        if let Some((&job_id, _)) = extras.iter().find(|(_, e)| e.job_key == job_key) {
            drop(extras);
            return self.load_job(job_id).await;
        }
        Ok(None)
    }

    async fn save_job(&self, job: &ScheduledJob) -> Result<(), SchedulerError> {
        let model = domain_to_model(job);

        scheduled_job::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(scheduled_job::Column::JobId)
                    .update_columns([
                        scheduled_job::Column::JobType,
                        scheduled_job::Column::ScheduleExpr,
                        scheduled_job::Column::Status,
                        scheduled_job::Column::LastRunAt,
                        scheduled_job::Column::NextRunAt,
                        scheduled_job::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        // Store supplementary fields
        let mut extras = self.extras.write().await;
        extras.insert(job.job_id, JobExtras {
            job_key: job.job_key.clone(),
            service_name: job.service_name.clone(),
            description: job.description.clone(),
            schedule: job.schedule.clone(),
            last_result: job.last_result.clone(),
        });

        Ok(())
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let results = ScheduledJobEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        self.enrich_results(results).await
    }

    async fn list_jobs_by_service(&self, service: &str) -> Result<Vec<ScheduledJob>, SchedulerError> {
        // Filter by service name from extras (since service_name isn't in entity)
        let extras = self.extras.read().await;
        let job_ids: Vec<Uuid> = extras.iter()
            .filter(|(_, e)| e.service_name == service || e.service_name == "all")
            .map(|(&id, _)| id)
            .collect();
        drop(extras);

        if job_ids.is_empty() {
            // Fallback: return all jobs and filter in-memory
            return self.list_jobs().await.map(|jobs| {
                jobs.into_iter().filter(|j| j.service_name == service || j.service_name == "all").collect()
            });
        }

        let results = ScheduledJobEntity::find()
            .filter(ScheduledJobColumn::JobId.is_in(job_ids))
            .all(&self.db)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        self.enrich_results(results).await
    }

    async fn list_active_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let results = ScheduledJobEntity::find()
            .filter(ScheduledJobColumn::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        self.enrich_results(results).await
    }

    async fn save_execution(&self, execution: &JobExecution) -> Result<(), SchedulerError> {
        self.executions.write().await.push(execution.clone());
        Ok(())
    }

    async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError> {
        let execs = self.executions.read().await;
        Ok(execs.iter().filter(|e| e.job_id == job_id).cloned().collect())
    }

    async fn acquire_lease(&self, job_key: &str, leader_id: &str, ttl_secs: u64) -> Result<bool, SchedulerError> {
        let mut leases = self.leases.write().await;
        let now = Utc::now();
        if let Some(lease) = leases.get(job_key) {
            if lease.leader_id != leader_id && lease.expires_at > now {
                return Ok(false);
            }
        }
        leases.insert(job_key.into(), LeaderLease {
            leader_id: leader_id.into(),
            job_key: job_key.into(),
            acquired_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_secs as i64),
        });
        Ok(true)
    }

    async fn release_lease(&self, job_key: &str, leader_id: &str) -> Result<(), SchedulerError> {
        let mut leases = self.leases.write().await;
        if let Some(lease) = leases.get(job_key) {
            if lease.leader_id == leader_id {
                leases.remove(job_key);
            }
        }
        Ok(())
    }
}

// ─── Conversion helpers ───────────────────────────────────────────────────────

fn domain_to_model(job: &ScheduledJob) -> scheduled_job::ActiveModel {
    let schedule_expr = match &job.schedule {
        JobSchedule::Every { interval_seconds } => format!("every:{}", interval_seconds),
        JobSchedule::Cron { expression } => format!("cron:{}", expression),
        JobSchedule::OnceAt { run_at } => format!("once:{}", run_at.timestamp()),
    };

    let payload_json = serde_json::json!({
        "job_key": job.job_key,
        "service_name": job.service_name,
        "description": job.description,
    }).to_string();

    scheduled_job::ActiveModel {
        job_id: sea_orm::ActiveValue::Set(job.job_id),
        job_type: sea_orm::ActiveValue::Set(job.job_type.to_string()),
        schedule_expr: sea_orm::ActiveValue::Set(schedule_expr),
        payload_json: sea_orm::ActiveValue::Set(payload_json),
        status: sea_orm::ActiveValue::Set(match job.status {
            JobStatus::Active => "active",
            JobStatus::Paused => "paused",
            JobStatus::Disabled => "disabled",
        }.into()),
        max_retries: sea_orm::ActiveValue::Set(3),
        retry_count: sea_orm::ActiveValue::Set(0),
        last_run_at: sea_orm::ActiveValue::Set(job.last_run_at),
        next_run_at: sea_orm::ActiveValue::Set(job.next_run_at),
        created_at: sea_orm::ActiveValue::Set(job.created_at),
        updated_at: sea_orm::ActiveValue::Set(job.updated_at),
    }
}

fn model_to_domain(m: scheduled_job::Model, extra: Option<JobExtras>) -> Result<ScheduledJob, SchedulerError> {
    // Fallback: parse payload_json when extras are not cached (e.g., after restart)
    let extra = extra.or_else(|| {
        serde_json::from_str::<serde_json::Value>(&m.payload_json).ok().map(|v| JobExtras {
            job_key: v.get("job_key").and_then(|s| s.as_str()).unwrap_or(&format!("job_{}", m.job_id)).to_string(),
            service_name: v.get("service_name").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            description: v.get("description").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            schedule: JobSchedule::Every { interval_seconds: 3600 },
            last_result: None,
        })
    }).unwrap_or_else(|| JobExtras {
        job_key: format!("job_{}", m.job_id),
        service_name: String::new(),
        description: String::new(),
        schedule: JobSchedule::Every { interval_seconds: 3600 },
        last_result: None,
    });
    let job_type = match m.job_type.as_str() {
        "subscription_renewal" => JobType::SubscriptionRenewal,
        "dunning_retry" => JobType::DunningRetry,
        "settlement_polling" => JobType::SettlementPolling,
        "invoice_overdue" => JobType::InvoiceOverdue,
        "ai_re_embedding" => JobType::AiReEmbedding,
        "exception_aging" => JobType::ExceptionAging,
        "auth_expiry_sweep" => JobType::AuthExpirySweep,
        "stuck_authorizing" => JobType::StuckAuthorizing,
        "data_retention" => JobType::DataRetention,
        "outbox_relay_health" => JobType::OutboxRelayHealth,
        "audit_hash_verify" => JobType::AuditHashVerify,
        "fee_variance_report" => JobType::FeeVarianceReport,
        "chargeback_deadline" => JobType::ChargebackDeadline,
        other => JobType::Custom(other.into()),
    };

    let status = match m.status.as_str() {
        "active" => JobStatus::Active,
        "paused" => JobStatus::Paused,
        "disabled" => JobStatus::Disabled,
        other => return Err(SchedulerError::DatabaseError(format!("Invalid status: {other}"))),
    };

    Ok(ScheduledJob {
        job_id: m.job_id,
        job_key: extra.job_key,
        service_name: extra.service_name,
        description: extra.description,
        schedule: extra.schedule,
        job_type,
        status,
        last_run_at: m.last_run_at,
        last_result: extra.last_result,
        next_run_at: m.next_run_at,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

impl PostgresSchedulerRepository {
    async fn enrich_results(&self, results: Vec<scheduled_job::Model>) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let extras = self.extras.read().await;
        results.into_iter().map(|m| {
            let extra = extras.get(&m.job_id).cloned();
            model_to_domain(m, extra)
        }).collect()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_job() -> ScheduledJob {
        let now = Utc::now();
        ScheduledJob {
            job_id: Uuid::now_v7(),
            job_key: "test_job".into(),
            service_name: "test-service".into(),
            description: "A test job".into(),
            schedule: JobSchedule::Every { interval_seconds: 60 },
            job_type: JobType::DataRetention,
            status: JobStatus::Active,
            last_run_at: None,
            last_result: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_domain_to_model() {
        let job = sample_job();
        let model = domain_to_model(&job);

        assert_eq!(model.job_id.unwrap(), job.job_id);
        assert_eq!(model.job_type.unwrap(), "data_retention");
        assert_eq!(model.status.unwrap(), "active");
        assert!(model.schedule_expr.unwrap().starts_with("every:60"));
    }

    #[tokio::test]
    async fn test_model_to_domain() {
        let job = sample_job();
        let entity = scheduled_job::Model {
            job_id: job.job_id,
            job_type: "data_retention".into(),
            schedule_expr: "every:60".into(),
            payload_json: r##"{"job_key":"test_job","service_name":"test-service","description":"A test job"}"##.into(),
            status: "active".into(),
            max_retries: 3,
            retry_count: 0,
            last_run_at: None,
            next_run_at: None,
            created_at: job.created_at,
            updated_at: job.updated_at,
        };
        let extra = Some(JobExtras {
            job_key: job.job_key.clone(),
            service_name: job.service_name.clone(),
            description: job.description.clone(),
            schedule: job.schedule.clone(),
            last_result: None,
        });

        let domain = model_to_domain(entity, extra).unwrap();
        assert_eq!(domain.job_id, job.job_id);
        assert_eq!(domain.job_key, "test_job");
        assert_eq!(domain.job_type, JobType::DataRetention);
        assert_eq!(domain.status, JobStatus::Active);
    }

    #[tokio::test]
    async fn test_custom_job_type() {
        let now = Utc::now();
        let entity = scheduled_job::Model {
            job_id: Uuid::now_v7(),
            job_type: "my_custom_job".into(),
            schedule_expr: "every:120".into(),
            payload_json: "{}".into(),
            status: "active".into(),
            max_retries: 3,
            retry_count: 0,
            last_run_at: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        };
        let extra = Some(JobExtras {
            job_key: "custom".into(),
            service_name: "custom-svc".into(),
            description: "Custom".into(),
            schedule: JobSchedule::Every { interval_seconds: 120 },
            last_result: None,
        });

        let domain = model_to_domain(entity, extra).unwrap();
        assert_eq!(domain.job_type, JobType::Custom("my_custom_job".into()));
    }

    #[tokio::test]
    async fn test_payload_json_fallback() {
        let now = Utc::now();
        // Model with populated payload_json but no extras cache (simulates restart)
        let entity = scheduled_job::Model {
            job_id: Uuid::now_v7(),
            job_type: "data_retention".into(),
            schedule_expr: "every:300".into(),
            payload_json: r##"{"job_key":"restored_job","service_name":"restored-svc","description":"Restored from payload"}"##.into(),
            status: "active".into(),
            max_retries: 3,
            retry_count: 0,
            last_run_at: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        };
        // No extras provided — should fall back to payload_json
        let domain = model_to_domain(entity, None).unwrap();
        assert_eq!(domain.job_key, "restored_job");
        assert_eq!(domain.service_name, "restored-svc");
        assert_eq!(domain.description, "Restored from payload");
    }
}
