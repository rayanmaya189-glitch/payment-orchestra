pub struct LeaderElection {
    lock_key: String,
    ttl_secs: u64,
    identity: String,
    client: redis::Client,
}

impl LeaderElection {
    pub fn new(client: redis::Client, lock_key: &str, ttl_secs: u64, identity: &str) -> Self {
        Self { client, lock_key: lock_key.to_string(), ttl_secs, identity: identity.to_string() }
    }

    pub fn try_acquire(&self) -> Result<bool, redis::RedisError> {
        let mut conn = self.client.get_connection()?;
        let result: Option<String> = redis::cmd("SET")
            .arg(&self.lock_key)
            .arg(&self.identity)
            .arg("NX")
            .arg("EX")
            .arg(self.ttl_secs)
            .query(&mut conn)?;
        Ok(result.is_some())
    }

    pub fn release(&self) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_connection()?;
        let _: Option<String> = redis::cmd("DEL")
            .arg(&self.lock_key)
            .query(&mut conn)?;
        Ok(())
    }
}
