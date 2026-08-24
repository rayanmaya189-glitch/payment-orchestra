//! Scheduler Service TDD tests

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

fn setup() -> SchedulerPipeline { SchedulerPipeline::new() }

#[tokio::test]
async fn test_register_job() {
    let pipeline = setup();
    let job = pipeline.api.register_job(RegisterJob {
        job_key: "auth_expiry_sweep".into(),
        service_name: "orchestration-service".into(),
        description: "Sweep expired authorizations".into(),
        schedule: JobSchedule::Every { interval_seconds: 300 },
        job_type: JobType::AuthExpirySweep,
    }).await.unwrap();

    assert_eq!(job.job_key, "auth_expiry_sweep");
    assert_eq!(job.status, JobStatus::Active);
    assert!(job.last_run_at.is_none());
}

#[tokio::test]
async fn test_register_duplicate_job_rejected() {
    let pipeline = setup();
    pipeline.api.register_job(RegisterJob {
        job_key: "test_job".into(),
        service_name: "test".into(),
        description: "Test".into(),
        schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("test".into()),
    }).await.unwrap();

    let result = pipeline.api.register_job(RegisterJob {
        job_key: "test_job".into(),
        service_name: "test".into(),
        description: "Duplicate".into(),
        schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("test".into()),
    }).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_job() {
    let pipeline = setup();
    let job = pipeline.api.register_job(RegisterJob {
        job_key: "test_exec".into(),
        service_name: "test".into(),
        description: "Test execution".into(),
        schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("test".into()),
    }).await.unwrap();

    let execution = pipeline.api.execute_job(ExecuteJob {
        job_id: job.job_id,
        success: true,
        duration_ms: 150,
        error_message: None,
    }).await.unwrap();

    assert_eq!(execution.status, ExecutionStatus::Completed);
    assert_eq!(execution.duration_ms, Some(150));
}

#[tokio::test]
async fn test_execute_disabled_job_rejected() {
    let pipeline = setup();
    let job = pipeline.api.register_job(RegisterJob {
        job_key: "disabled_test".into(),
        service_name: "test".into(),
        description: "Test disabled".into(),
        schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("test".into()),
    }).await.unwrap();

    let paused = pipeline.api.pause_job(PauseJob { job_id: job.job_id }).await.unwrap();
    assert_eq!(paused.status, JobStatus::Paused);

    let result = pipeline.api.execute_job(ExecuteJob {
        job_id: job.job_id,
        success: true,
        duration_ms: 100,
        error_message: None,
    }).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pause_and_resume_job() {
    let pipeline = setup();
    let job = pipeline.api.register_job(RegisterJob {
        job_key: "pause_resume".into(),
        service_name: "test".into(),
        description: "Pause/resume test".into(),
        schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("test".into()),
    }).await.unwrap();

    let paused = pipeline.api.pause_job(PauseJob { job_id: job.job_id }).await.unwrap();
    assert_eq!(paused.status, JobStatus::Paused);

    let resumed = pipeline.api.resume_job(ResumeJob { job_id: job.job_id }).await.unwrap();
    assert_eq!(resumed.status, JobStatus::Active);
}

#[tokio::test]
async fn test_list_jobs() {
    let pipeline = setup();
    pipeline.api.register_job(RegisterJob {
        job_key: "job_a".into(), service_name: "svc_a".into(),
        description: "A".into(), schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("a".into()),
    }).await.unwrap();
    pipeline.api.register_job(RegisterJob {
        job_key: "job_b".into(), service_name: "svc_b".into(),
        description: "B".into(), schedule: JobSchedule::Every { interval_seconds: 120 },
        job_type: JobType::Custom("b".into()),
    }).await.unwrap();

    let jobs = pipeline.api.list_jobs().await.unwrap();
    assert_eq!(jobs.len(), 2);
}

#[tokio::test]
async fn test_leader_election() {
    let pipeline = setup();
    let acquired = pipeline.api.acquire_leadership(AcquireLeadership {
        job_key: "test_job".into(),
        leader_id: "instance_1".into(),
        ttl_secs: 30,
    }).await.unwrap();
    assert!(acquired);

    // Second instance should not acquire
    let second = pipeline.api.acquire_leadership(AcquireLeadership {
        job_key: "test_job".into(),
        leader_id: "instance_2".into(),
        ttl_secs: 30,
    }).await.unwrap();
    assert!(!second);
}

#[tokio::test]
async fn test_leader_election_release_and_reacquire() {
    let pipeline = setup();
    pipeline.api.acquire_leadership(AcquireLeadership {
        job_key: "lease_test".into(),
        leader_id: "instance_1".into(),
        ttl_secs: 30,
    }).await.unwrap();

    pipeline.api.release_leadership(ReleaseLeadership {
        job_key: "lease_test".into(),
        leader_id: "instance_1".into(),
    }).await.unwrap();

    let reacquired = pipeline.api.acquire_leadership(AcquireLeadership {
        job_key: "lease_test".into(),
        leader_id: "instance_2".into(),
        ttl_secs: 30,
    }).await.unwrap();
    assert!(reacquired);
}

#[tokio::test]
async fn test_list_executions() {
    let pipeline = setup();
    let job = pipeline.api.register_job(RegisterJob {
        job_key: "exec_test".into(), service_name: "test".into(),
        description: "Execution list test".into(),
        schedule: JobSchedule::Every { interval_seconds: 60 },
        job_type: JobType::Custom("test".into()),
    }).await.unwrap();

    pipeline.api.execute_job(ExecuteJob {
        job_id: job.job_id, success: true, duration_ms: 50, error_message: None,
    }).await.unwrap();
    pipeline.api.execute_job(ExecuteJob {
        job_id: job.job_id, success: false, duration_ms: 200, error_message: Some("timeout".into()),
    }).await.unwrap();

    let executions = pipeline.api.list_executions(job.job_id).await.unwrap();
    assert_eq!(executions.len(), 2);
}

#[tokio::test]
async fn test_list_default_jobs() {
    let pipeline = setup();
    let default_jobs = pipeline.api.list_default_jobs().await;
    assert_eq!(default_jobs.len(), 10);
    assert!(default_jobs.iter().any(|(k, _, _, _)| k == "auth_expiry_sweep"));
}

#[tokio::test]
async fn test_nonexistent_job_returns_not_found() {
    let pipeline = setup();
    let result = pipeline.api.get_job(Uuid::now_v7()).await;
    assert!(result.is_err());
}
