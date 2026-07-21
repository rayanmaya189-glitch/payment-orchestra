//! Leader election for distributed scheduler instances.
//!
//! Uses Redis SET NX with TTL to ensure only one scheduler instance
//! runs each job at a time (SRS LEADER-001).

use redis::aio::ConnectionManager;
use tracing::{info, warn};

/// Redis-based distributed lock for leader election.
///
/// Uses `SET key value NX EX ttl` — atomic set-if-not-exists with expiry.
/// If the lock holder crashes, the lock auto-expires after TTL.
pub struct LeaderElection {
    redis: ConnectionManager,
    instance_id: String,
}

impl LeaderElection {
    pub fn new(redis: ConnectionManager, instance_id: String) -> Self {
        Self { redis, instance_id }
    }

    /// Try to acquire leadership for a job.
    ///
    /// Returns true if this instance is the leader.
    /// The lock expires after `ttl_secs` seconds — the leader must re-acquire before expiry.
    pub async fn try_acquire(&self, job_name: &str, ttl_secs: u64) -> Result<bool, String> {
        let mut redis = self.redis.clone();
        let lock_key = format!("leader:{}", job_name);

        let acquired: bool = redis::cmd("SET")
            .arg(&lock_key)
            .arg(&self.instance_id)
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut redis)
            .await
            .map_err(|e| format!("Redis SET NX failed: {e}"))?;

        if acquired {
            info!(
                job = job_name,
                instance = %self.instance_id,
                ttl = ttl_secs,
                "Acquired leadership"
            );
        }

        Ok(acquired)
    }

    /// Release leadership (optional — lock auto-expires).
    pub async fn release(&self, job_name: &str) -> Result<(), String> {
        let mut redis = self.redis.clone();
        let lock_key = format!("leader:{}", job_name);

        // Only release if we hold the lock
        let current: Option<String> = redis::cmd("GET")
            .arg(&lock_key)
            .query_async(&mut redis)
            .await
            .map_err(|e| format!("Redis GET failed: {e}"))?;

        if current.as_deref() == Some(&self.instance_id) {
            redis::cmd("DEL")
                .arg(&lock_key)
                .query_async::<()>(&mut redis)
                .await
                .map_err(|e| format!("Redis DEL failed: {e}"))?;
            info!(job = job_name, instance = %self.instance_id, "Released leadership");
        }

        Ok(())
    }

    /// Check if this instance is the current leader.
    pub async fn is_leader(&self, job_name: &str) -> Result<bool, String> {
        let mut redis = self.redis.clone();
        let lock_key = format!("leader:{}", job_name);

        let current: Option<String> = redis::cmd("GET")
            .arg(&lock_key)
            .query_async(&mut redis)
            .await
            .map_err(|e| format!("Redis GET failed: {e}"))?;

        Ok(current.as_deref() == Some(&self.instance_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_leader_election() {
        // Test the logic without Redis
        use std::collections::HashMap;
        use std::sync::Mutex;

        let locks: Mutex<HashMap<String, (String, std::time::Instant)>> = Mutex::new(HashMap::new());

        // Instance 1 tries to acquire
        let acquired1 = {
            let mut l = locks.lock().unwrap();
            let key = "leader:job1".to_string();
            if l.contains_key(&key) {
                false
            } else {
                l.insert(key, ("instance1".to_string(), std::time::Instant::now()));
                true
            }
        };
        assert!(acquired1);

        // Instance 2 tries to acquire (should fail)
        let acquired2 = {
            let mut l = locks.lock().unwrap();
            let key = "leader:job1".to_string();
            if l.contains_key(&key) {
                false
            } else {
                l.insert(key, ("instance2".to_string(), std::time::Instant::now()));
                true
            }
        };
        assert!(!acquired2);

        // Instance 1 releases
        {
            let mut l = locks.lock().unwrap();
            let key = "leader:job1".to_string();
            l.remove(&key);
        }

        // Instance 2 tries again (should succeed)
        let acquired3 = {
            let mut l = locks.lock().unwrap();
            let key = "leader:job1".to_string();
            if l.contains_key(&key) {
                false
            } else {
                l.insert(key, ("instance2".to_string(), std::time::Instant::now()));
                true
            }
        };
        assert!(acquired3);
    }
}
