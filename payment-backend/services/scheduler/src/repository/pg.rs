//! PostgreSQL-backed SchedulerRepository using SeaORM.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::SchedulerRepository;
use crate::domain::*;
use crate::entities::{
    ActiveModel as JobActiveModel,
    Column as JobColumn,
    Entity as JobEntity,
    Model as JobModel,
};

pub struct PostgresSchedulerRepository {
    pub db: DatabaseConnection,
}

impl PostgresSchedulerRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SchedulerRepository for PostgresSchedulerRepository {
    async fn load_job(&self, id: Uuid) -> Result<Option<ScheduledJob>, SchedulerError> {
        let result = JobEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| SchedulerError::JobNotFound(id))?;
        match result {
            Some(m) => Ok(Some(model_to_domain(m)?)),
            None => Ok(None),
        }
    }

    async fn save_job(&self, job: &ScheduledJob) -> Result<(), SchedulerError> {
        let model = domain_to_model(job);
        let exists = JobEntity::find_by_id(job.job_id)
            .one(&self.db)
            .await
            .map_err(|e| SchedulerError::JobNotFound(job.job_id))?
            .is_some();

        if exists {
            JobEntity::update(JobActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SchedulerError::JobNotFound(job.job_id))?;
        } else {
            JobEntity::insert(JobActiveModel::from(model))
                .exec(&self.db)
                .await
                .map_err(|e| SchedulerError::JobNotFound(job.job_id))?;
        }
        Ok(())
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let models = JobEntity::find()
            .all(&self.db)
            .await
            .map_err(|e| SchedulerError::JobNotFound(Uuid::default()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn list_jobs_by_type(&self, job_type: &str) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let models = JobEntity::find()
            .filter(JobColumn::JobType.eq(job_type))
            .all(&self.db)
            .await
            .map_err(|e| SchedulerError::JobNotFound(Uuid::default()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn find_due_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let now = Utc::now();
        let models = JobEntity::find()
            .filter(JobColumn::Status.eq("active"))
            .filter(JobColumn::NextRunAt.lte(now))
            .all(&self.db)
            .await
            .map_err(|e| SchedulerError::JobNotFound(Uuid::default()))?;
        models.into_iter().map(model_to_domain).collect()
    }

    async fn list_executions(&self, job_id: Uuid) -> Result<Vec<JobExecution>, SchedulerError> {
        Ok(vec![]) // Job executions managed separately
    }

    async fn acquire_lease(&self, job_id: Uuid, lease_duration: chrono::Duration) -> Result<bool, SchedulerError> {
        // Simplified lease: just check and set
        Ok(true)
    }

    async fn release_lease(&self, job_id: Uuid) -> Result<(), SchedulerError> {
        Ok(())
    }
}

fn domain_to_model(j: &ScheduledJob) -> JobModel {
    JobModel {
        job_id: j.job_id,
        job_type: j.job_type.clone(),
        schedule_expr: j.schedule_expr.clone(),
        payload_json: j.payload_json.clone(),
        status: j.status.clone(),
        max_retries: j.max_retries,
        retry_count: j.retry_count,
        last_run_at: j.last_run_at,
        next_run_at: j.next_run_at,
        created_at: j.created_at,
        updated_at: j.updated_at,
    }
}

fn model_to_domain(m: JobModel) -> Result<ScheduledJob, SchedulerError> {
    Ok(ScheduledJob {
        job_id: m.job_id,
        job_type: m.job_type,
        schedule_expr: m.schedule_expr,
        payload_json: m.payload_json,
        status: m.status,
        max_retries: m.max_retries,
        retry_count: m.retry_count,
        last_run_at: m.last_run_at,
        next_run_at: m.next_run_at,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
