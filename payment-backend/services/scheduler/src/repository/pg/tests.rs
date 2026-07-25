//! Tests for Scheduler PostgreSQL repository.

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
